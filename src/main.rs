#![deny(clippy::pedantic, clippy::nursery)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::match_same_arms)]
#![allow(clippy::missing_const_for_fn)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::significant_drop_tightening)]

mod db;
mod fedimint;
mod routes;
mod ui_components;
mod util;

use std::collections::{BTreeMap, VecDeque};
use std::fmt::Debug;
use std::sync::Arc;

use db::Database;

use fedimint::{FederationView, Wallet};
use fedimint_core::config::FederationId;
use iced::futures::{SinkExt, StreamExt};
use iced::widget::{column, container, row, scrollable, Theme};
use iced::window::settings::PlatformSpecific;
use iced::window::Settings;
use iced::{Element, Length, Size, Task};
use nip_55::nip_46::{Nip46OverNip55ServerStream, Nip46RequestApproval};
use nostr_sdk::PublicKey;
use routes::{Loadable, Route, RouteName};
use ui_components::sidebar;

fn main() -> iced::Result {
    tracing_subscriber::fmt::init();

    iced::application("Keystache", Keystache::update, Keystache::view)
        .subscription(Keystache::subscription)
        .theme(|_| Theme::Dark)
        .window(Settings {
            size: iced::Size {
                width: 800.0,
                height: 600.0,
            },
            position: iced::window::Position::Default,
            min_size: Some(Size {
                width: 600.0,
                height: 400.0,
            }),
            max_size: None,
            visible: true,
            resizable: true,
            decorations: true,
            transparent: false,
            level: iced::window::Level::Normal,
            icon: None,                                     // TODO: Set icon.
            platform_specific: PlatformSpecific::default(), // TODO: Set platform specific settings for each platform.
            exit_on_close_request: true,
        })
        .run()
}

struct Keystache {
    page: Route,
}

impl Default for Keystache {
    fn default() -> Self {
        Self {
            page: Route::new_locked(),
        }
    }
}

impl Keystache {
    fn update(&mut self, event: KeystacheMessage) -> Task<KeystacheMessage> {
        self.page.update(event)
    }

    fn view(&self) -> Element<KeystacheMessage> {
        let Self { page, .. } = self;

        let mut content: Element<KeystacheMessage> = Element::new(scrollable(
            container(column![page.view()].spacing(20).padding(20)).center_x(Length::Fill),
        ));

        if page.to_name() != RouteName::Unlock {
            content = Element::new(row![sidebar(self), content]);
        };

        container(content).center_y(Length::Fill).into()
    }

    fn subscription(&self) -> iced::Subscription<KeystacheMessage> {
        let Some(connected_state) = self.page.get_connected_state() else {
            return iced::Subscription::none();
        };

        let db_clone = connected_state.db.clone();

        let wallet = connected_state.wallet.clone();

        let wallet_sub = iced::subscription::channel(
            std::any::TypeId::of::<Wallet>(),
            100,
            |mut output| async move {
                loop {
                    let mut wallet_update_stream = wallet.get_update_stream();

                    while let Some(views) = wallet_update_stream.next().await {
                        output
                            .send(KeystacheMessage::FederationViewsUpdate { views })
                            .await
                            .unwrap();
                    }
                }
            },
        );

        let nip46_sub = iced::subscription::channel(
            std::any::TypeId::of::<Nip46OverNip55ServerStream>(),
            100,
            |mut output| async move {
                loop {
                    let mut stream = Nip46OverNip55ServerStream::start(
                        "/tmp/nip55-kind24133.sock",
                        db_clone.clone(),
                    )
                    .unwrap();

                    while let Some((request_list, public_key, response_sender)) =
                        stream.next().await
                    {
                        output
                            .send(KeystacheMessage::IncomingNip46Request(Arc::new((
                                request_list,
                                public_key,
                                response_sender,
                            ))))
                            .await
                            .unwrap();
                    }
                }
            },
        );

        iced::subscription::Subscription::batch(vec![nip46_sub, wallet_sub])
    }
}

#[derive(Debug, Clone)]
enum KeystacheMessage {
    Navigate(RouteName),
    NavigateHomeAndSetConnectedState(ConnectedState),

    UnlockPage(routes::unlock::Message),
    NostrKeypairsPage(routes::nostr_keypairs::Message),
    NostrRelaysPage(routes::nostr_relays::Message),
    BitcoinWalletPage(routes::bitcoin_wallet::Message),
    SettingsPage(routes::settings::Message),

    DbDeleteAllData,

    FederationViewsUpdate {
        views: BTreeMap<FederationId, FederationView>,
    },

    IncomingNip46Request(
        Arc<(
            Vec<nostr_sdk::nips::nip46::Request>,
            PublicKey,
            iced::futures::channel::oneshot::Sender<Nip46RequestApproval>,
        )>,
    ),
    ApproveFirstIncomingNip46Request,
    RejectFirstIncomingNip46Request,
}

#[derive(Clone)]
struct ConnectedState {
    db: Arc<Database>,
    wallet: Arc<Wallet>,
    #[allow(clippy::type_complexity)]
    in_flight_nip46_requests: VecDeque<
        Arc<(
            Vec<nostr_sdk::nips::nip46::Request>,
            PublicKey,
            iced::futures::channel::oneshot::Sender<Nip46RequestApproval>,
        )>,
    >,
    loadable_federation_views: Loadable<BTreeMap<FederationId, FederationView>>,
}

// TODO: Clean up this implementation.
impl Debug for ConnectedState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ConnectedState")
    }
}
