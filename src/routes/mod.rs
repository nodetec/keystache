use std::{collections::VecDeque, str::FromStr, sync::Arc};

use bitcoin_wallet::{MaybeLoadingFederationConfig, ParsedFederationInviteCodeState};
use fedimint_core::invite_code::InviteCode;
use iced::{
    widget::{column, row, text, Column, Text},
    Alignment, Element, Task,
};
use nip_55::nip_46::Nip46RequestApproval;
use nostr_sdk::{
    secp256k1::{Keypair, Secp256k1},
    SecretKey,
};

use crate::{
    db::Database,
    ui_components::{icon_button, PaletteColor, SvgIcon},
    ConnectedState, KeystacheMessage,
};

mod bitcoin_wallet;
mod home;
pub mod nostr_keypairs;
pub mod nostr_relays;
mod settings;
mod unlock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteName {
    Unlock,
    Home,
    NostrKeypairs(nostr_keypairs::SubrouteName),
    NostrRelays(nostr_relays::SubrouteName),
    BitcoinWallet,
    Settings,
}

impl RouteName {
    pub fn is_same_top_level_route_as(self, other: Self) -> bool {
        match self {
            Self::Unlock => other == Self::Unlock,
            Self::Home => other == Self::Home,
            Self::NostrKeypairs(_) => matches!(other, Self::NostrKeypairs(_)),
            Self::NostrRelays(_) => matches!(other, Self::NostrRelays(_)),
            Self::BitcoinWallet => other == Self::BitcoinWallet,
            Self::Settings => other == Self::Settings,
        }
    }
}

#[derive(Clone)]
pub enum Route {
    Unlock(unlock::Page),
    Home(home::Page),
    NostrKeypairs(nostr_keypairs::Page),
    NostrRelays(nostr_relays::Page),
    BitcoinWallet(bitcoin_wallet::Page),
    Settings(settings::Page),
}

impl Route {
    pub fn new_locked() -> Self {
        Self::Unlock(unlock::Page {
            password: String::new(),
            is_secure: true,
            db_already_exists: Database::exists(),
        })
    }

    pub fn to_name(&self) -> RouteName {
        match self {
            Self::Unlock(_) => RouteName::Unlock,
            Self::Home(_) => RouteName::Home,
            Self::NostrKeypairs(nostr_keypairs) => {
                RouteName::NostrKeypairs(nostr_keypairs.subroute.to_name())
            }
            Self::NostrRelays(nostr_relays) => {
                RouteName::NostrRelays(nostr_relays.subroute.to_name())
            }
            Self::BitcoinWallet(_) => RouteName::BitcoinWallet,
            Self::Settings(_) => RouteName::Settings,
        }
    }

    // TODO: Remove this clippy allow.
    #[allow(clippy::too_many_lines)]
    #[allow(clippy::cognitive_complexity)]
    pub fn update(&mut self, msg: KeystacheMessage) -> Task<KeystacheMessage> {
        match msg {
            KeystacheMessage::Navigate(route_name) => {
                let new_self_or = match route_name {
                    RouteName::Unlock => Some(Self::new_locked()),
                    RouteName::Home => self.get_connected_state().map(|connected_state| {
                        Self::Home(home::Page {
                            connected_state: connected_state.clone(),
                        })
                    }),
                    RouteName::NostrKeypairs(subroute_name) => {
                        self.get_connected_state()
                            .map(|connected_state| match subroute_name {
                                nostr_keypairs::SubrouteName::List => {
                                    Self::NostrKeypairs(nostr_keypairs::Page {
                                        connected_state: connected_state.clone(),
                                        subroute: nostr_keypairs::Subroute::List(
                                            nostr_keypairs::List {},
                                        ),
                                    })
                                }
                                nostr_keypairs::SubrouteName::Add => {
                                    Self::NostrKeypairs(nostr_keypairs::Page {
                                        connected_state: connected_state.clone(),
                                        subroute: nostr_keypairs::Subroute::Add(
                                            nostr_keypairs::Add {
                                                nsec: String::new(),
                                                keypair_or: None,
                                            },
                                        ),
                                    })
                                }
                            })
                    }
                    RouteName::NostrRelays(subroute_name) => {
                        self.get_connected_state()
                            .map(|connected_state| match subroute_name {
                                nostr_relays::SubrouteName::List => {
                                    Self::NostrRelays(nostr_relays::Page {
                                        connected_state: connected_state.clone(),
                                        subroute: nostr_relays::Subroute::List(
                                            nostr_relays::List {},
                                        ),
                                    })
                                }
                                nostr_relays::SubrouteName::Add => {
                                    Self::NostrRelays(nostr_relays::Page {
                                        connected_state: connected_state.clone(),
                                        subroute: nostr_relays::Subroute::Add(nostr_relays::Add {
                                            websocket_url: String::new(),
                                        }),
                                    })
                                }
                            })
                    }
                    RouteName::BitcoinWallet => self.get_connected_state().map(|connected_state| {
                        Self::BitcoinWallet(bitcoin_wallet::Page {
                            connected_state: connected_state.clone(),
                            federation_invite_code: String::new(),
                            parsed_federation_invite_code_state_or: None,
                        })
                    }),
                    RouteName::Settings => self.get_connected_state().map(|connected_state| {
                        Self::Settings(settings::Page {
                            connected_state: connected_state.clone(),
                        })
                    }),
                };

                if let Some(new_self_or) = new_self_or {
                    *self = new_self_or;
                } else {
                    // TODO: Log warning that navigation failed.
                }

                Task::none()
            }
            KeystacheMessage::UnlockPasswordInputChanged(new_password) => {
                if let Self::Unlock(unlock::Page { password, .. }) = self {
                    *password = new_password;
                }

                Task::none()
            }
            KeystacheMessage::UnlockToggleSecureInput => {
                if let Self::Unlock(unlock::Page { is_secure, .. }) = self {
                    *is_secure = !*is_secure;
                }

                Task::none()
            }
            KeystacheMessage::UnlockPasswordSubmitted => {
                if let Self::Unlock(unlock::Page { password, .. }) = self {
                    if let Ok(db) = Database::open_or_create_in_app_data_dir(password) {
                        let db = Arc::new(db);

                        *self = Self::Home(home::Page {
                            connected_state: ConnectedState {
                                db,
                                in_flight_nip46_requests: VecDeque::new(),
                            },
                        });
                    }
                }

                Task::none()
            }
            KeystacheMessage::DbDeleteAllData => {
                if let Self::Unlock(unlock::Page {
                    db_already_exists, ..
                }) = self
                {
                    Database::delete();
                    *db_already_exists = false;
                }

                Task::none()
            }
            KeystacheMessage::SaveKeypair(keypair) => {
                if let Some(connected_state) = self.get_connected_state_mut() {
                    // TODO: Surface this error to the UI.
                    let _ = connected_state.db.save_keypair(&keypair);
                }

                Task::none()
            }
            KeystacheMessage::SaveKeypairNsecInputChanged(new_nsec) => {
                if let Self::NostrKeypairs(nostr_keypairs::Page {
                    subroute:
                        nostr_keypairs::Subroute::Add(nostr_keypairs::Add {
                            nsec, keypair_or, ..
                        }),
                    ..
                }) = self
                {
                    *nsec = new_nsec;

                    // Set `keypair_or` to `Some` if `nsec` is a valid secret key, `None` otherwise.
                    *keypair_or = SecretKey::from_str(nsec).map_or(None, |secret_key| {
                        Some(Keypair::from_secret_key(&Secp256k1::new(), &secret_key))
                    });
                }

                Task::none()
            }
            KeystacheMessage::DeleteKeypair { public_key } => {
                if let Some(connected_state) = self.get_connected_state_mut() {
                    // TODO: Surface this error to the UI.
                    _ = connected_state.db.remove_keypair(&public_key);
                }

                Task::none()
            }
            KeystacheMessage::SaveRelay { websocket_url } => {
                if let Some(connected_state) = self.get_connected_state_mut() {
                    // TODO: Surface this error to the UI.
                    let _ = connected_state.db.save_relay(websocket_url);
                }

                Task::none()
            }
            KeystacheMessage::SaveRelayWebsocketUrlInputChanged(new_websocket_url) => {
                if let Self::NostrRelays(nostr_relays::Page {
                    subroute: nostr_relays::Subroute::Add(nostr_relays::Add { websocket_url }),
                    ..
                }) = self
                {
                    *websocket_url = new_websocket_url;
                }

                Task::none()
            }
            KeystacheMessage::DeleteRelay { websocket_url } => {
                if let Some(connected_state) = self.get_connected_state_mut() {
                    // TODO: Surface this error to the UI.
                    _ = connected_state.db.remove_relay(&websocket_url);
                }

                Task::none()
            }
            KeystacheMessage::JoinFederationInviteCodeInputChanged(new_federation_invite_code) => {
                if let Self::BitcoinWallet(bitcoin_wallet::Page {
                    federation_invite_code,
                    parsed_federation_invite_code_state_or,
                    ..
                }) = self
                {
                    *federation_invite_code = new_federation_invite_code;

                    if let Ok(invite_code) = InviteCode::from_str(federation_invite_code) {
                        *parsed_federation_invite_code_state_or =
                            Some(ParsedFederationInviteCodeState {
                                invite_code: invite_code.clone(),
                                maybe_loading_federation_config:
                                    MaybeLoadingFederationConfig::Loading,
                            });

                        Task::perform(
                            async move {
                                match fedimint_api_client::download_from_invite_code(&invite_code).await {
                                    Ok(config) => {
                                        KeystacheMessage::LoadedFederationConfigFromInviteCode {
                                            config_invite_code:   invite_code,
                                            config,
                                        }
                                    }
                                    // TODO: Include error in message and display it in the UI.
                                    Err(_err) => {
                                        KeystacheMessage::FailedToLoadFederationConfigFromInviteCode { config_invite_code: invite_code }
                                    }
                                }
                            },
                            |msg| msg,
                        )
                    } else {
                        *parsed_federation_invite_code_state_or = None;

                        Task::none()
                    }
                } else {
                    Task::none()
                }
            }
            KeystacheMessage::LoadedFederationConfigFromInviteCode {
                config_invite_code,
                config,
            } => {
                if let Self::BitcoinWallet(bitcoin_wallet::Page {
                    parsed_federation_invite_code_state_or:
                        Some(ParsedFederationInviteCodeState {
                            invite_code,
                            maybe_loading_federation_config,
                        }),
                    ..
                }) = self
                {
                    // If the invite code has changed since the request was made, ignore the response.
                    if &config_invite_code == invite_code {
                        *maybe_loading_federation_config =
                            MaybeLoadingFederationConfig::Loaded(config);
                    }
                }

                Task::none()
            }
            KeystacheMessage::FailedToLoadFederationConfigFromInviteCode { config_invite_code } => {
                if let Self::BitcoinWallet(bitcoin_wallet::Page {
                    parsed_federation_invite_code_state_or:
                        Some(ParsedFederationInviteCodeState {
                            invite_code,
                            maybe_loading_federation_config,
                        }),
                    ..
                }) = self
                {
                    // If the invite code has changed since the request was made, ignore the response.
                    // Also only update the state if the config hasn't already been loaded.
                    if &config_invite_code == invite_code
                        && matches!(
                            maybe_loading_federation_config,
                            MaybeLoadingFederationConfig::Loading
                        )
                    {
                        *maybe_loading_federation_config = MaybeLoadingFederationConfig::Failed;
                    }
                }

                Task::none()
            }
            KeystacheMessage::JoinFedimintFederation(_invite_code) => {
                // TODO: Implement this.

                Task::none()
            }
            KeystacheMessage::IncomingNip46Request(data) => {
                if let Some(connected_state) = self.get_connected_state_mut() {
                    connected_state.in_flight_nip46_requests.push_back(data);
                }

                Task::none()
            }
            KeystacheMessage::ApproveFirstIncomingNip46Request => {
                if let Some(connected_state) = self.get_connected_state_mut() {
                    if let Some(req) = connected_state.in_flight_nip46_requests.pop_front() {
                        let req = Arc::try_unwrap(req).unwrap();
                        req.2.send(Nip46RequestApproval::Approve).unwrap();
                    }
                }

                Task::none()
            }
            KeystacheMessage::RejectFirstIncomingNip46Request => {
                if let Some(connected_state) = self.get_connected_state_mut() {
                    if let Some(req) = connected_state.in_flight_nip46_requests.pop_front() {
                        let req = Arc::try_unwrap(req).unwrap();
                        req.2.send(Nip46RequestApproval::Reject).unwrap();
                    }
                }

                Task::none()
            }
        }
    }

    pub fn view(&self) -> Element<KeystacheMessage> {
        // If there are any incoming NIP46 requests, display the first one over the rest of the UI.
        if let Some(connected_state) = self.get_connected_state() {
            if let Some(req) = connected_state.in_flight_nip46_requests.front() {
                return Column::new()
                    .push(Text::new("Incoming NIP-46 request"))
                    .push(Text::new(format!("{:?}", req.0)))
                    .push(
                        row![
                            icon_button("Approve", SvgIcon::ThumbUp, PaletteColor::Primary)
                                .on_press(KeystacheMessage::ApproveFirstIncomingNip46Request),
                            icon_button("Reject", SvgIcon::ThumbDown, PaletteColor::Primary)
                                .on_press(KeystacheMessage::RejectFirstIncomingNip46Request),
                        ]
                        .spacing(20),
                    )
                    .align_items(Alignment::Center)
                    .into();
            }
        }

        match self {
            Self::Unlock(unlock) => unlock.view(),
            Self::Home(home) => home.view(),
            Self::NostrKeypairs(nostr_keypairs) => nostr_keypairs.view(),
            Self::NostrRelays(nostr_relays) => nostr_relays.view(),
            Self::BitcoinWallet(bitcoin_wallet) => bitcoin_wallet.view(),
            Self::Settings(settings) => settings.view(),
        }
        .into()
    }

    pub fn get_connected_state(&self) -> Option<&ConnectedState> {
        match self {
            Self::Unlock { .. } => None,
            Self::Home(home::Page { connected_state }) => Some(connected_state),
            Self::NostrKeypairs(nostr_keypairs::Page {
                connected_state, ..
            }) => Some(connected_state),
            Self::NostrRelays(nostr_relays::Page {
                connected_state, ..
            }) => Some(connected_state),
            Self::BitcoinWallet(bitcoin_wallet::Page {
                connected_state, ..
            }) => Some(connected_state),
            Self::Settings(settings::Page { connected_state }) => Some(connected_state),
        }
    }

    fn get_connected_state_mut(&mut self) -> Option<&mut ConnectedState> {
        match self {
            Self::Unlock { .. } => None,
            Self::Home(home::Page { connected_state }) => Some(connected_state),
            Self::NostrKeypairs(nostr_keypairs::Page {
                connected_state, ..
            }) => Some(connected_state),
            Self::NostrRelays(nostr_relays::Page {
                connected_state, ..
            }) => Some(connected_state),
            Self::BitcoinWallet(bitcoin_wallet::Page {
                connected_state, ..
            }) => Some(connected_state),
            Self::Settings(settings::Page { connected_state }) => Some(connected_state),
        }
    }
}

pub fn container<'a>(title: &str) -> Column<'a, KeystacheMessage> {
    column![text(title.to_string()).size(35)]
        .spacing(20)
        .align_items(iced::Alignment::Center)
}
