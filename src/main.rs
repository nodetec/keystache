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

use app::App;

use fedimint::Wallet;
use iced::widget::Theme;
use iced::window::settings::PlatformSpecific;
use iced::window::Settings;
use iced::Size;

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
