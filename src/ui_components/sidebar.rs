use iced::widget::container::Style;
use iced::widget::{column, container, vertical_space};
use iced::Border;
use iced::{Alignment, Element, Shadow};

use crate::routes::{nostr_keypairs, nostr_relays, settings, RouteName};
use crate::{Keystache, KeystacheMessage};

use super::util::lighten;
use super::{sidebar_button, SvgIcon};

pub fn sidebar(keystache: &Keystache) -> Element<KeystacheMessage> {
    let sidebar = container(
        column![
            sidebar_button("Home", SvgIcon::Home, RouteName::Home, keystache)
                .on_press(KeystacheMessage::Navigate(RouteName::Home)),
            sidebar_button(
                "Keys",
                SvgIcon::Key,
                RouteName::NostrKeypairs(nostr_keypairs::SubrouteName::List),
                keystache
            )
            .on_press(KeystacheMessage::Navigate(RouteName::NostrKeypairs(
                nostr_keypairs::SubrouteName::List
            ))),
            sidebar_button(
                "Relays",
                SvgIcon::Hub,
                RouteName::NostrRelays(nostr_relays::SubrouteName::List),
                keystache
            )
            .on_press(KeystacheMessage::Navigate(RouteName::NostrRelays(
                nostr_relays::SubrouteName::List
            ))),
            sidebar_button(
                "Wallet",
                SvgIcon::CurrencyBitcoin,
                RouteName::BitcoinWallet,
                keystache
            )
            .on_press(KeystacheMessage::Navigate(RouteName::BitcoinWallet)),
            vertical_space(),
            sidebar_button(
                "Settings",
                SvgIcon::Settings,
                RouteName::Settings(settings::SubrouteName::Main),
                keystache
            )
            .on_press(KeystacheMessage::Navigate(RouteName::Settings(
                settings::SubrouteName::Main
            ))),
        ]
        .spacing(8)
        .align_items(Alignment::Start),
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
