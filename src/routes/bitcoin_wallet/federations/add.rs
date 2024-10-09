use std::str::FromStr;

use fedimint_core::{
    config::{ClientConfig, META_FEDERATION_NAME_KEY},
    invite_code::InviteCode,
};
use iced::{
    widget::{text_input, Column, Text},
    Task,
};

use crate::{
    app,
    routes::{self, bitcoin_wallet, container, ConnectedState, Loadable, RouteName},
    ui_components::{icon_button, PaletteColor, SvgIcon, Toast, ToastStatus},
    util::truncate_text,
};

#[derive(Debug, Clone)]
pub enum Message {
    JoinFederationInviteCodeInputChanged(String),

    LoadedFederationConfigFromInviteCode {
        // The invite code that was used to load the federation config.
        config_invite_code: InviteCode,
        // The loaded federation config.
        config: ClientConfig,
    },
    FailedToLoadFederationConfigFromInviteCode {
        // The invite code that was used to attempt to load the federation config.
        config_invite_code: InviteCode,
    },

    JoinFederation(InviteCode),
    JoinedFederation(InviteCode),
}

pub struct Page {
    connected_state: ConnectedState,
    federation_invite_code: String,
    parsed_federation_invite_code_state_or: Option<ParsedFederationInviteCodeState>,
}

pub struct ParsedFederationInviteCodeState {
    invite_code: InviteCode,
    loadable_federation_config: Loadable<ClientConfig>,
}

impl Page {
    pub fn new(connected_state: &ConnectedState) -> Self {
        Self {
            connected_state: connected_state.clone(),
            federation_invite_code: String::new(),
            parsed_federation_invite_code_state_or: None,
        }
    }

    pub fn update(&mut self, msg: Message) -> Task<app::Message> {
        match msg {
            Message::JoinFederationInviteCodeInputChanged(new_federation_invite_code) => {
                self.federation_invite_code = new_federation_invite_code;

                if let Ok(invite_code) = InviteCode::from_str(&self.federation_invite_code) {
                    self.parsed_federation_invite_code_state_or =
                        Some(ParsedFederationInviteCodeState {
                            invite_code: invite_code.clone(),
                            loadable_federation_config: Loadable::Loading,
                        });

                    Task::perform(
                        async move {
                            match fedimint_api_client::download_from_invite_code(&invite_code).await
                            {
                                Ok(config) => {
                                    app::Message::Routes(routes::Message::BitcoinWalletPage(
                                        bitcoin_wallet::Message::Federations(super::Message::Add(
                                            Message::LoadedFederationConfigFromInviteCode {
                                                config_invite_code: invite_code,
                                                config,
                                            },
                                        )),
                                    ))
                                }
                                // TODO: Include error in message and display it in the UI.
                                Err(_err) => {
                                    app::Message::Routes(routes::Message::BitcoinWalletPage(
                                        bitcoin_wallet::Message::Federations(super::Message::Add(
                                            Message::FailedToLoadFederationConfigFromInviteCode {
                                                config_invite_code: invite_code,
                                            },
                                        )),
                                    ))
                                }
                            }
                        },
                        |msg| msg,
                    )
                } else {
                    self.parsed_federation_invite_code_state_or = None;

                    Task::none()
                }
            }
            Message::LoadedFederationConfigFromInviteCode {
                config_invite_code,
                config,
            } => {
                if let Some(ParsedFederationInviteCodeState {
                    invite_code,
                    loadable_federation_config,
                }) = &mut self.parsed_federation_invite_code_state_or
                {
                    // If the invite code has changed since the request was made, ignore the response.
                    if &config_invite_code == invite_code {
                        *loadable_federation_config = Loadable::Loaded(config);
                    }
                }

                Task::none()
            }
            Message::FailedToLoadFederationConfigFromInviteCode { config_invite_code } => {
                if let Some(ParsedFederationInviteCodeState {
                    invite_code,
                    loadable_federation_config,
                }) = &mut self.parsed_federation_invite_code_state_or
                {
                    // If the invite code has changed since the request was made, ignore the response.
                    // Also only update the state if the config hasn't already been loaded.
                    if &config_invite_code == invite_code
                        && matches!(loadable_federation_config, Loadable::Loading)
                    {
                        *loadable_federation_config = Loadable::Failed;
                    }
                }

                Task::none()
            }
            Message::JoinFederation(invite_code) => {
                let wallet = self.connected_state.wallet.clone();

                Task::stream(async_stream::stream! {
                    match wallet.join_federation(invite_code.clone()).await {
                        Ok(()) => {
                            yield app::Message::AddToast(Toast {
                                title: "Joined federation".to_string(),
                                body: "You have successfully joined the federation.".to_string(),
                                status: ToastStatus::Good,
                            });

                            yield app::Message::Routes(
                                routes::Message::BitcoinWalletPage(
                                    bitcoin_wallet::Message::Federations(
                                        super::Message::Add(
                                            Message::JoinedFederation(invite_code)
                                        )
                                    )
                                )
                            );
                        }
                        Err(err) => {
                            yield app::Message::AddToast(Toast {
                                title: "Failed to join federation".to_string(),
                                body: format!("Failed to join the federation: {err}"),
                                status: ToastStatus::Bad,
                            });
                        }
                    }
                })
            }
            Message::JoinedFederation(invite_code) => {
                // If the invite code matches the one that was just joined, navigate back to the `Main` page.
                if let Some(invite_code_state) = &self.parsed_federation_invite_code_state_or {
                    if invite_code_state.invite_code == invite_code {
                        return Task::done(app::Message::Routes(routes::Message::Navigate(
                            RouteName::BitcoinWallet(bitcoin_wallet::SubrouteName::Main),
                        )));
                    }
                }

                Task::none()
            }
        }
    }

    pub fn view<'a>(&self) -> Column<'a, app::Message> {
        let mut container = container("Join Federation")
            .push(
                text_input("Federation Invite Code", &self.federation_invite_code)
                    .on_input(|input| {
                        app::Message::Routes(routes::Message::BitcoinWalletPage(
                            bitcoin_wallet::Message::Federations(super::Message::Add(
                                Message::JoinFederationInviteCodeInputChanged(input),
                            )),
                        ))
                    })
                    .padding(10)
                    .size(30),
            )
            .push(
                icon_button("Join Federation", SvgIcon::Groups, PaletteColor::Primary)
                    .on_press_maybe(self.parsed_federation_invite_code_state_or.as_ref().map(
                        |parsed_federation_invite_code_state| {
                            app::Message::Routes(routes::Message::BitcoinWalletPage(
                                bitcoin_wallet::Message::Federations(super::Message::Add(
                                    Message::JoinFederation(
                                        parsed_federation_invite_code_state.invite_code.clone(),
                                    ),
                                )),
                            ))
                        },
                    )),
            );

        if let Some(parsed_federation_invite_code_state) =
            &self.parsed_federation_invite_code_state_or
        {
            container = container
                .push(Text::new("Federation ID").size(25))
                .push(Text::new(truncate_text(
                    &parsed_federation_invite_code_state
                        .invite_code
                        .federation_id()
                        .to_string(),
                    21,
                    true,
                )));

            match &parsed_federation_invite_code_state.loadable_federation_config {
                Loadable::Loading => {
                    container = container.push(Text::new("Loading..."));
                }
                Loadable::Loaded(client_config) => {
                    container = container
                        .push(Text::new("Federation Name").size(25))
                        .push(Text::new(
                            client_config
                                .meta::<String>(META_FEDERATION_NAME_KEY)
                                .ok()
                                .flatten()
                                .unwrap_or_default(),
                        ))
                        .push(Text::new("Modules").size(25))
                        .push(Text::new(
                            client_config
                                .modules
                                .values()
                                .map(|module| module.kind().to_string())
                                .collect::<Vec<_>>()
                                .join(", "),
                        ))
                        .push(Text::new("Guardians").size(25));
                    for peer_url in client_config.global.api_endpoints.values() {
                        container = container
                            .push(Text::new(format!("{} ({})", peer_url.name, peer_url.url)));
                    }
                }
                Loadable::Failed => {
                    container = container.push(Text::new("Failed to load client config"));
                }
            }
        }

        container = container.push(
            icon_button("Back", SvgIcon::ArrowBack, PaletteColor::Background).on_press(
                app::Message::Routes(routes::Message::Navigate(RouteName::BitcoinWallet(
                    bitcoin_wallet::SubrouteName::Main,
                ))),
            ),
        );

        container
    }
}
