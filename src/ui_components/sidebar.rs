use iced::widget::container::Style;
use iced::widget::{column, container, vertical_space};
use iced::Border;
use iced::{Alignment, Element, Shadow};

use crate::routes::{bitcoin_wallet, nostr_keypairs, nostr_relays, settings, RouteName};
use crate::{app, routes};

use super::{sidebar_button, SvgIcon};
use crate::util::lighten;

pub fn sidebar(keystache: &app::App) -> Element<app::Message> {
    let sidebar = container(
        column![
            sidebar_button("Home", SvgIcon::Home, &RouteName::Home, keystache).on_press(
                app::Message::Routes(routes::Message::Navigate(RouteName::Home))
            ),
            sidebar_button(
                "Keys",
                SvgIcon::Key,
                &RouteName::NostrKeypairs(nostr_keypairs::SubrouteName::List),
                keystache
            )
            .on_press(app::Message::Routes(routes::Message::Navigate(
                RouteName::NostrKeypairs(nostr_keypairs::SubrouteName::List)
            ))),
            sidebar_button(
                "Relays",
                SvgIcon::Hub,
                &RouteName::NostrRelays(nostr_relays::SubrouteName::List),
                keystache
            )
            .on_press(app::Message::Routes(routes::Message::Navigate(
                RouteName::NostrRelays(nostr_relays::SubrouteName::List)
            ))),
            sidebar_button(
                "Wallet",
                SvgIcon::CurrencyBitcoin,
                &RouteName::BitcoinWallet(bitcoin_wallet::SubrouteName::List),
                keystache
            )
            .on_press(app::Message::Routes(routes::Message::Navigate(
                RouteName::BitcoinWallet(bitcoin_wallet::SubrouteName::List)
            ))),
            vertical_space(),
            sidebar_button(
                "Settings",
                SvgIcon::Settings,
                &RouteName::Settings(settings::SubrouteName::Main),
                keystache
            )
            .on_press(app::Message::Routes(routes::Message::Navigate(
                RouteName::Settings(settings::SubrouteName::Main)
            ))),
        ]
        .spacing(8)
        .align_x(Alignment::Start),
    )
    .padding(8)
    .style(|theme| -> Style {
        Style {
            text_color: None,
            background: Some(lighten(theme.palette().background, 0.05).into()),
            border: Border::default(),
            shadow: Shadow::default(),
        }
    });
    sidebar.into()
}
