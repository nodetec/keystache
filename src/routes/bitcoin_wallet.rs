use std::{collections::BTreeMap, str::FromStr};

use fedimint_core::{
    config::{ClientConfig, FederationId, META_FEDERATION_NAME_KEY},
    invite_code::InviteCode,
};
use iced::{
    widget::{text_input, Column, Text},
    Task,
};

use crate::{
    fedimint::FederationView,
    ui_components::{icon_button, PaletteColor, SvgIcon},
    util::{format_amount_sats, truncate_text},
    ConnectedState, KeystacheMessage,
};

use super::{container, Loadable};

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

    LoadFederationConfigViews,
    LoadedFederationConfigViews {
        views: BTreeMap<FederationId, FederationView>,
    },
}

#[derive(Clone)]
pub struct Page {
    pub connected_state: ConnectedState,
    pub federation_invite_code: String,
    pub parsed_federation_invite_code_state_or: Option<ParsedFederationInviteCodeState>,
    pub loadable_federation_views: Loadable<BTreeMap<FederationId, FederationView>>,
}

#[derive(Clone)]
pub struct ParsedFederationInviteCodeState {
    pub invite_code: InviteCode,
    pub loadable_federation_config: Loadable<ClientConfig>,
}

impl Page {
    pub fn update(&mut self, msg: Message) -> Task<KeystacheMessage> {
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
                    loadable_federation_config: maybe_loading_federation_config,
                }) = &mut self.parsed_federation_invite_code_state_or
                {
                    // If the invite code has changed since the request was made, ignore the response.
                    if &config_invite_code == invite_code {
                        *maybe_loading_federation_config = Loadable::Loaded(config);
                    }
                }

                Task::none()
            }
            Message::FailedToLoadFederationConfigFromInviteCode { config_invite_code } => {
                if let Some(ParsedFederationInviteCodeState {
                    invite_code,
                    loadable_federation_config: maybe_loading_federation_config,
                }) = &mut self.parsed_federation_invite_code_state_or
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
            Message::LoadFederationConfigViews => {
                let wallet = self.connected_state.wallet.clone();

                Task::future(async move {
                    let views = wallet.get_current_state().await;

                    KeystacheMessage::BitcoinWalletPage(Message::LoadedFederationConfigViews {
                        views,
                    })
                })
            }
            Message::LoadedFederationConfigViews { views } => {
                self.loadable_federation_views = Loadable::Loaded(views);

                Task::none()
            }
        }
    }

    // TODO: Remove this clippy allow.
    #[allow(clippy::too_many_lines)]
    pub fn view<'a>(&self) -> Column<'a, KeystacheMessage> {
        let mut container = container("Wallet");

        match &self.loadable_federation_views {
            Loadable::Loading => {
                container = container.push(Text::new("Loading federation config views..."));
            }
            Loadable::Loaded(views) => {
                container =
                    container.push(Column::new().push(Text::new("Federation Config Views")));

                for (federation_id, view) in views {
                    let mut column = Column::new()
                        .push(Text::new(federation_id.to_string()))
                        .push(Text::new(
                            view.name_or
                                .clone()
                                .unwrap_or_else(|| "Unnamed Federation".to_string()),
                        ))
                        .push(Text::new(format_amount_sats(view.balance.msats / 1000)))
                        .push(Text::new("Gateways").size(25));

                    for gateway in &view.gateways {
                        column = column.push(Text::new(gateway.info.gateway_id.to_string()));
                    }

                    container = container.push(column);
                }
            }
            Loadable::Failed => {
                container = container.push(Text::new("Failed to load federation config views."));
            }
        }

        container = container
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

        container
    }
}
