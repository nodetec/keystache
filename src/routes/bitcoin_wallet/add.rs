use std::str::FromStr;
use std::sync::Arc;

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
    fedimint::Wallet,
    routes::{self, container, ConnectedState, Loadable},
    ui_components::{icon_button, PaletteColor, SvgIcon, Toast, ToastStatus},
    util::truncate_text,
};

#[derive(Debug, Clone)]
pub enum Message {
    InviteCodeInputChanged(String),

    LoadedFederationConfigFromInviteCode {
        // The invite code that was used to load the federation config.
        invite_code: InviteCode,
        // The loaded federation config from the federation that the invite code belongs to.
        config: ClientConfig,
    },
    FailedToLoadFederationConfigFromInviteCode {
        // The invite code that was used to attempt to load the federation config.
        invite_code: InviteCode,
    },

    JoinFederation(InviteCode),
    JoinedFederation(InviteCode),
}

pub struct Page {
    wallet: Arc<Wallet>,
    invite_code_input: String,
    parsed_invite_code_state_or: Option<ParsedInviteCodeState>,
}

struct ParsedInviteCodeState {
    invite_code: InviteCode,
    loadable_federation_config: Loadable<ClientConfig>,
}

impl Page {
    pub fn new(connected_state: &ConnectedState) -> Self {
        Self {
            wallet: connected_state.wallet.clone(),
            invite_code_input: String::new(),
            parsed_invite_code_state_or: None,
        }
    }

    // TODO: Remove this clippy allow.
    #[allow(clippy::too_many_lines)]
    pub fn update(&mut self, msg: Message) -> Task<app::Message> {
        match msg {
            Message::InviteCodeInputChanged(new_invite_code_input) => {
                self.invite_code_input = new_invite_code_input;

                let Ok(invite_code) = InviteCode::from_str(&self.invite_code_input) else {
                    self.parsed_invite_code_state_or = None;

                    return Task::none();
                };

                self.parsed_invite_code_state_or = Some(ParsedInviteCodeState {
                    invite_code: invite_code.clone(),
                    loadable_federation_config: Loadable::Loading,
                });

                Task::future(async {
                    match fedimint_api_client::download_from_invite_code(&invite_code).await {
                        Ok(config) => app::Message::Routes(routes::Message::BitcoinWalletPage(
                            super::Message::Add(Message::LoadedFederationConfigFromInviteCode {
                                invite_code,
                                config,
                            }),
                        )),
                        Err(_err) => app::Message::Routes(routes::Message::BitcoinWalletPage(
                            super::Message::Add(
                                Message::FailedToLoadFederationConfigFromInviteCode { invite_code },
                            ),
                        )),
                    }
                })
            }
            Message::LoadedFederationConfigFromInviteCode {
                invite_code,
                config,
            } => {
                if let Some(ParsedInviteCodeState {
                    invite_code: parsed_invite_code,
                    loadable_federation_config,
                }) = &mut self.parsed_invite_code_state_or
                {
                    // If the invite code has changed since the request was made, ignore the response.
                    if &invite_code == parsed_invite_code {
                        *loadable_federation_config = Loadable::Loaded(config);
                    }
                }

                Task::none()
            }
            Message::FailedToLoadFederationConfigFromInviteCode { invite_code } => {
                if let Some(ParsedInviteCodeState {
                    invite_code: parsed_invite_code,
                    loadable_federation_config,
                }) = &mut self.parsed_invite_code_state_or
                {
                    // If the invite code has changed since the request was made, ignore the response.
                    // Also only update the state if the user attempted to load the config.
                    if &invite_code == parsed_invite_code
                        && matches!(loadable_federation_config, Loadable::Loading)
                    {
                        *loadable_federation_config = Loadable::Failed;
                    }
                }

                // TODO: Show toast instead of returning an empty task.
                Task::none()
            }
            Message::JoinFederation(invite_code) => {
                let wallet = self.wallet.clone();

                Task::stream(async_stream::stream! {
                    match wallet.join_federation(invite_code.clone()).await {
                        Ok(()) => {
                            yield app::Message::AddToast(Toast {
                                title: "Joined federation".to_string(),
                                body: "You have successfully joined the federation.".to_string(),
                                status: ToastStatus::Good,
                            });

                            yield app::Message::Routes(routes::Message::BitcoinWalletPage(super::Message::Add(
                                Message::JoinedFederation(invite_code)
                            )));
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
                // If the invite code matches the one that was just joined, navigate back to the `List` page.
                if let Some(invite_code_state) = &self.parsed_invite_code_state_or {
                    if invite_code_state.invite_code == invite_code {
                        return Task::done(app::Message::Routes(routes::Message::Navigate(
                            routes::RouteName::BitcoinWallet(super::SubrouteName::List),
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
                text_input("Federation Invite Code", &self.invite_code_input)
                    .on_input(|input| {
                        app::Message::Routes(routes::Message::BitcoinWalletPage(
                            super::Message::Add(Message::InviteCodeInputChanged(input)),
                        ))
                    })
                    .padding(10)
                    .size(30),
            )
            .push(
                icon_button("Join Federation", SvgIcon::Groups, PaletteColor::Primary)
                    .on_press_maybe(self.parsed_invite_code_state_or.as_ref().map(
                        |parsed_federation_invite_code_state| {
                            app::Message::Routes(routes::Message::BitcoinWalletPage(
                                super::Message::Add(Message::JoinFederation(
                                    parsed_federation_invite_code_state.invite_code.clone(),
                                )),
                            ))
                        },
                    )),
            );

        if let Some(parsed_federation_invite_code_state) = &self.parsed_invite_code_state_or {
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
                app::Message::Routes(routes::Message::Navigate(super::RouteName::BitcoinWallet(
                    super::SubrouteName::List,
                ))),
            ),
        );

        container
    }
}
