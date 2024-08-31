use std::str::FromStr;

use fedimint_core::{
    config::{ClientConfig, META_FEDERATION_NAME_KEY},
    invite_code::InviteCode,
};
use iced::{
    widget::{container::Style, text_input, Column, Container, Text},
    Border, Length, Shadow, Task, Theme,
};

use crate::{
    ui_components::{icon_button, PaletteColor, SvgIcon},
    util::lighten,
    util::{format_amount, truncate_text},
    ConnectedState, KeystacheMessage,
};

use super::{container, Loadable, RouteName};

#[derive(Debug, Clone)]
pub enum Message {
    JoinFederationInviteCodeInputChanged(String),

    LoadedFederationConfigFromInviteCode {
        // The invite code that was used to load the federation config.
        config_invite_code: InviteCode,
        // The loaded federation config.
        config: fedimint_core::config::ClientConfig,
    },
    FailedToLoadFederationConfigFromInviteCode {
        // The invite code that was used to attempt to load the federation config.
        config_invite_code: InviteCode,
    },

    JoinFedimintFederation(InviteCode),
    ConnectedToFederation,
}

#[derive(Clone)]
pub struct Page {
    pub connected_state: ConnectedState,
    pub subroute: Subroute,
}

impl Page {
    pub fn update(&mut self, msg: Message) -> Task<KeystacheMessage> {
        match msg {
            Message::JoinFederationInviteCodeInputChanged(new_federation_invite_code) => {
                let Subroute::Add(Add {
                    federation_invite_code,
                    parsed_federation_invite_code_state_or,
                }) = &mut self.subroute
                else {
                    return Task::none();
                };

                *federation_invite_code = new_federation_invite_code;

                if let Ok(invite_code) = InviteCode::from_str(federation_invite_code) {
                    *parsed_federation_invite_code_state_or =
                        Some(ParsedFederationInviteCodeState {
                            invite_code: invite_code.clone(),
                            loadable_federation_config: Loadable::Loading,
                        });

                    Task::perform(
                        async move {
                            match fedimint_api_client::download_from_invite_code(&invite_code).await
                            {
                                Ok(config) => KeystacheMessage::BitcoinWalletPage(
                                    Message::LoadedFederationConfigFromInviteCode {
                                        config_invite_code: invite_code,
                                        config,
                                    },
                                ),
                                // TODO: Include error in message and display it in the UI.
                                Err(_err) => KeystacheMessage::BitcoinWalletPage(
                                    Message::FailedToLoadFederationConfigFromInviteCode {
                                        config_invite_code: invite_code,
                                    },
                                ),
                            }
                        },
                        |msg| msg,
                    )
                } else {
                    *parsed_federation_invite_code_state_or = None;

                    Task::none()
                }
            }
            Message::LoadedFederationConfigFromInviteCode {
                config_invite_code,
                config,
            } => {
                let Subroute::Add(Add {
                    parsed_federation_invite_code_state_or,
                    ..
                }) = &mut self.subroute
                else {
                    return Task::none();
                };

                if let Some(ParsedFederationInviteCodeState {
                    invite_code,
                    loadable_federation_config: maybe_loading_federation_config,
                }) = parsed_federation_invite_code_state_or
                {
                    // If the invite code has changed since the request was made, ignore the response.
                    if &config_invite_code == invite_code {
                        *maybe_loading_federation_config = Loadable::Loaded(config);
                    }
                }

                Task::none()
            }
            Message::FailedToLoadFederationConfigFromInviteCode { config_invite_code } => {
                let Subroute::Add(Add {
                    parsed_federation_invite_code_state_or,
                    ..
                }) = &mut self.subroute
                else {
                    return Task::none();
                };

                if let Some(ParsedFederationInviteCodeState {
                    invite_code,
                    loadable_federation_config: maybe_loading_federation_config,
                }) = parsed_federation_invite_code_state_or
                {
                    // If the invite code has changed since the request was made, ignore the response.
                    // Also only update the state if the config hasn't already been loaded.
                    if &config_invite_code == invite_code
                        && matches!(maybe_loading_federation_config, Loadable::Loading)
                    {
                        *maybe_loading_federation_config = Loadable::Failed;
                    }
                }

                Task::none()
            }
            Message::JoinFedimintFederation(invite_code) => {
                let wallet = self.connected_state.wallet.clone();

                Task::future(async move {
                    wallet.join_federation(invite_code).await.unwrap();
                    KeystacheMessage::BitcoinWalletPage(Message::ConnectedToFederation)
                })
            }
            Message::ConnectedToFederation => {
                // TODO: Do something here, or remove `ConnectedToFederation` message variant.

                Task::none()
            }
        }
    }

    pub fn view<'a>(&self) -> Column<'a, KeystacheMessage> {
        match &self.subroute {
            Subroute::List(list) => list.view(&self.connected_state),
            Subroute::Add(add) => add.view(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubrouteName {
    List,
    Add,
}

impl SubrouteName {
    pub fn to_default_subroute(&self) -> Subroute {
        match self {
            Self::List => Subroute::List(List {}),
            Self::Add => Subroute::Add(Add {
                federation_invite_code: String::new(),
                parsed_federation_invite_code_state_or: None,
            }),
        }
    }
}

#[derive(Clone)]
pub enum Subroute {
    List(List),
    Add(Add),
}

impl Subroute {
    pub fn to_name(&self) -> SubrouteName {
        match self {
            Self::List(_) => SubrouteName::List,
            Self::Add(_) => SubrouteName::Add,
        }
    }
}

#[derive(Clone)]
pub struct List {}

impl List {
    fn view<'a>(&self, connected_state: &ConnectedState) -> Column<'a, KeystacheMessage> {
        let mut container = container("Wallet");

        match &connected_state.loadable_federation_views {
            Loadable::Loading => {
                container = container.push(Text::new("Loading federations...").size(25));
            }
            Loadable::Loaded(views) => {
                container = container.push(Column::new().push(Text::new("Federations").size(25)));

                for (federation_id, view) in views {
                    let mut column: Column<_, Theme, _> = Column::new()
                        .push(
                            Text::new(
                                view.name_or
                                    .clone()
                                    .unwrap_or_else(|| "Unnamed Federation".to_string()),
                            )
                            .size(25),
                        )
                        .push(Text::new(format!(
                            "Federation ID: {}",
                            truncate_text(&federation_id.to_string(), 23, true)
                        )))
                        .push(Text::new(format_amount(view.balance)))
                        .push(Text::new("Gateways").size(20));

                    for gateway in &view.gateways {
                        column = column.push(Text::new(truncate_text(
                            &gateway.info.gateway_id.to_string(),
                            23,
                            true,
                        )));
                    }

                    container = container.push(
                        Container::new(column)
                            .padding(10)
                            .width(Length::Fill)
                            .style(|theme| -> Style {
                                Style {
                                    text_color: None,
                                    background: Some(
                                        lighten(theme.palette().background, 0.05).into(),
                                    ),
                                    border: Border {
                                        color: iced::Color::WHITE,
                                        width: 0.0,
                                        radius: (8.0).into(),
                                    },
                                    shadow: Shadow::default(),
                                }
                            }),
                    );
                }
            }
            Loadable::Failed => {
                container =
                    container.push(Text::new("Failed to load federation config views.").size(25));
            }
        }

        container = container.push(
            icon_button("Join Federation", SvgIcon::Add, PaletteColor::Primary).on_press(
                KeystacheMessage::Navigate(RouteName::BitcoinWallet(SubrouteName::Add)),
            ),
        );

        container
    }
}

#[derive(Clone)]
pub struct Add {
    pub federation_invite_code: String,
    pub parsed_federation_invite_code_state_or: Option<ParsedFederationInviteCodeState>,
}

#[derive(Clone)]
pub struct ParsedFederationInviteCodeState {
    pub invite_code: InviteCode,
    pub loadable_federation_config: Loadable<ClientConfig>,
}

impl Add {
    fn view<'a>(&self) -> Column<'a, KeystacheMessage> {
        let mut container = container("Add Keypair")
            .push(
                text_input("Federation Invite Code", &self.federation_invite_code)
                    .on_input(|input| {
                        KeystacheMessage::BitcoinWalletPage(
                            Message::JoinFederationInviteCodeInputChanged(input),
                        )
                    })
                    .padding(10)
                    .size(30),
            )
            .push(
                icon_button("Join Federation", SvgIcon::Groups, PaletteColor::Primary)
                    .on_press_maybe(self.parsed_federation_invite_code_state_or.as_ref().map(
                        |parsed_federation_invite_code_state| {
                            KeystacheMessage::BitcoinWalletPage(Message::JoinFedimintFederation(
                                parsed_federation_invite_code_state.invite_code.clone(),
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
                KeystacheMessage::Navigate(RouteName::BitcoinWallet(SubrouteName::List)),
            ),
        );

        container
    }
}
