use std::{collections::VecDeque, fmt::Debug, sync::Arc};

use iced::{
    widget::{column, row, text, Column, Text},
    Alignment, Element, Task,
};
use nip_55::nip_46::Nip46RequestApproval;
use nostr_sdk::PublicKey;

use crate::{
    app,
    db::Database,
    fedimint::{Wallet, WalletView},
    nostr::{NostrModule, NostrState},
    ui_components::{icon_button, PaletteColor, SvgIcon},
};

pub mod bitcoin_wallet;
mod home;
pub mod nostr_keypairs;
pub mod nostr_relays;
pub mod settings;
pub mod unlock;

// TODO: Is this actually safe to clone? I think we need to wrap all fields in `Arc`s.
#[derive(Clone)]
pub struct ConnectedState {
    pub db: Arc<Database>,
    pub wallet: Arc<Wallet>,
    #[allow(clippy::type_complexity)]
    pub in_flight_nip46_requests: VecDeque<
        Arc<(
            Vec<nostr_sdk::nips::nip46::Request>,
            PublicKey,
            iced::futures::channel::oneshot::Sender<Nip46RequestApproval>,
        )>,
    >,
    pub loadable_wallet_view: Loadable<WalletView>,
    pub nostr_module: NostrModule,
    pub nostr_state: NostrState,
}

// TODO: Clean up this implementation.
impl Debug for ConnectedState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ConnectedState")
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    Navigate(RouteName),
    NavigateHomeAndSetConnectedState(ConnectedState),

    UnlockPage(unlock::Message),
    NostrKeypairsPage(nostr_keypairs::Message),
    NostrRelaysPage(nostr_relays::Message),
    BitcoinWalletPage(bitcoin_wallet::Message),
    SettingsPage(settings::Message),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouteName {
    Unlock,
    Home,
    NostrKeypairs(nostr_keypairs::SubrouteName),
    NostrRelays(nostr_relays::SubrouteName),
    BitcoinWallet(bitcoin_wallet::SubrouteName),
    Settings(settings::SubrouteName),
}

impl RouteName {
    pub fn is_same_top_level_route_as(&self, other: &Self) -> bool {
        match self {
            Self::Unlock => other == &Self::Unlock,
            Self::Home => other == &Self::Home,
            Self::NostrKeypairs(_) => matches!(other, Self::NostrKeypairs(_)),
            Self::NostrRelays(_) => matches!(other, Self::NostrRelays(_)),
            Self::BitcoinWallet(_) => matches!(other, Self::BitcoinWallet(_)),
            Self::Settings(_) => matches!(other, Self::Settings(_)),
        }
    }
}

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
            Self::BitcoinWallet(bitcoin_wallet) => {
                RouteName::BitcoinWallet(bitcoin_wallet.subroute.to_name())
            }
            Self::Settings(settings) => RouteName::Settings(settings.subroute.to_name()),
        }
    }

    pub fn update(&mut self, msg: Message) -> Task<app::Message> {
        match msg {
            Message::Navigate(route_name) => {
                let new_self_or = match route_name {
                    RouteName::Unlock => Some(Self::new_locked()),
                    RouteName::Home => self.get_connected_state().map(|connected_state| {
                        Self::Home(home::Page {
                            connected_state: connected_state.clone(),
                        })
                    }),
                    RouteName::NostrKeypairs(subroute_name) => {
                        self.get_connected_state().map(|connected_state| {
                            Self::NostrKeypairs(nostr_keypairs::Page {
                                connected_state: connected_state.clone(),
                                subroute: subroute_name.to_default_subroute(),
                            })
                        })
                    }
                    RouteName::NostrRelays(subroute_name) => {
                        self.get_connected_state().map(|connected_state| {
                            Self::NostrRelays(nostr_relays::Page {
                                connected_state: connected_state.clone(),
                                subroute: subroute_name.to_default_subroute(),
                            })
                        })
                    }
                    RouteName::BitcoinWallet(subroute_name) => {
                        self.get_connected_state().map(|connected_state| {
                            Self::BitcoinWallet(bitcoin_wallet::Page {
                                connected_state: connected_state.clone(),
                                subroute: subroute_name.to_default_subroute(connected_state),
                            })
                        })
                    }
                    RouteName::Settings(subroute_name) => {
                        self.get_connected_state().map(|connected_state| {
                            Self::Settings(settings::Page {
                                connected_state: connected_state.clone(),
                                subroute: subroute_name.to_default_subroute(),
                            })
                        })
                    }
                };

                if let Some(new_self) = new_self_or {
                    *self = new_self;
                } else {
                    // TODO: Log warning that navigation failed.
                }

                Task::none()
            }
            Message::NavigateHomeAndSetConnectedState(connected_state) => {
                *self = Self::Home(home::Page { connected_state });

                Task::none()
            }
            Message::UnlockPage(unlock_message) => {
                if let Self::Unlock(unlock_page) = self {
                    unlock_page.update(unlock_message)
                } else {
                    // TODO: Log a warning that the unlock page is not active.
                    Task::none()
                }
            }
            Message::NostrKeypairsPage(nostr_keypairs_message) => {
                if let Self::NostrKeypairs(nostr_keypairs_page) = self {
                    nostr_keypairs_page.update(nostr_keypairs_message)
                } else {
                    // TODO: Log a warning that the keypairs page is not active.
                    Task::none()
                }
            }
            Message::NostrRelaysPage(nostr_relays_message) => {
                if let Self::NostrRelays(nostr_relays_page) = self {
                    nostr_relays_page.update(nostr_relays_message)
                } else {
                    // TODO: Log a warning that the relays page is not active.
                    Task::none()
                }
            }
            Message::BitcoinWalletPage(bitcoin_wallet_message) => {
                if let Self::BitcoinWallet(bitcoin_wallet_page) = self {
                    bitcoin_wallet_page.update(bitcoin_wallet_message)
                } else {
                    // TODO: Log a warning that the bitcoin wallet page is not active.
                    Task::none()
                }
            }
            Message::SettingsPage(settings_message) => {
                if let Self::Settings(settings_page) = self {
                    settings_page.update(settings_message)
                } else {
                    // TODO: Log a warning that the settings page is not active.
                    Task::none()
                }
            }
        }
    }

    pub fn view(&self) -> Element<app::Message> {
        // If there are any incoming NIP46 requests, display the first one over the rest of the UI.
        if let Some(connected_state) = self.get_connected_state() {
            if let Some(req) = connected_state.in_flight_nip46_requests.front() {
                return Column::new()
                    .push(Text::new("Incoming NIP-46 request"))
                    .push(Text::new(format!("{:?}", req.0)))
                    .push(
                        row![
                            icon_button("Approve", SvgIcon::ThumbUp, PaletteColor::Primary)
                                .on_press(app::Message::ApproveFirstIncomingNip46Request),
                            icon_button("Reject", SvgIcon::ThumbDown, PaletteColor::Primary)
                                .on_press(app::Message::RejectFirstIncomingNip46Request),
                        ]
                        .spacing(20),
                    )
                    .align_x(Alignment::Center)
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
            Self::Settings(settings::Page {
                connected_state, ..
            }) => Some(connected_state),
        }
    }

    pub fn get_connected_state_mut(&mut self) -> Option<&mut ConnectedState> {
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
            Self::Settings(settings::Page {
                connected_state, ..
            }) => Some(connected_state),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Loadable<T> {
    Loading,
    Loaded(T),
    Failed,
}

impl<T> Loadable<T> {
    pub fn as_ref_option(&self) -> Option<&T> {
        match self {
            Self::Loaded(data) => Some(data),
            _ => None,
        }
    }
}

fn container<'a>(title: &str) -> Column<'a, app::Message> {
    column![text(title.to_string()).size(35)]
        .spacing(20)
        .align_x(iced::Alignment::Center)
}
