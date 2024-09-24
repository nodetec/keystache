#![deny(clippy::pedantic, clippy::nursery)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::match_same_arms)]
#![allow(clippy::missing_const_for_fn)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::significant_drop_tightening)]

mod app;
mod db;
mod fedimint;
mod nostr;
mod routes;
mod ui_components;
mod util;

use std::collections::{BTreeMap, VecDeque};
use std::fmt::Debug;
use std::sync::Arc;

use app::App;
use db::Database;

use fedimint::{FederationView, Wallet};
use fedimint_core::config::FederationId;
use iced::widget::Theme;
use iced::window::settings::PlatformSpecific;
use iced::window::Settings;
use iced::Size;
use nip_55::nip_46::Nip46RequestApproval;
use nostr::{NostrModule, NostrState};
use nostr_sdk::PublicKey;
use routes::Loadable;

fn main() -> iced::Result {
    tracing_subscriber::fmt::init();

    iced::application("Keystache", App::update, App::view)
        .subscription(App::subscription)
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
    nostr_module: NostrModule,
    nostr_state: NostrState,
}

// TODO: Clean up this implementation.
impl Debug for ConnectedState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ConnectedState")
    }
}
