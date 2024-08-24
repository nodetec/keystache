use iced::widget::container::Style;
use iced::widget::{column, container, vertical_space};
use iced::Border;
use iced::{Alignment, Element, Shadow};

use crate::routes::RouteName;
use crate::{Keystache, Message};

use super::util::lighten;
use super::{sidebar_button, SvgIcon};

pub fn sidebar(keystache: &Keystache) -> Element<Message> {
    let sidebar = container(
        column![
            sidebar_button("Home", SvgIcon::Home, RouteName::Home, keystache)
                .on_press(Message::Navigate(RouteName::Home)),
            sidebar_button("Keys", SvgIcon::Key, RouteName::AddNostrKeypair, keystache)
                .on_press(Message::Navigate(RouteName::AddNostrKeypair)),
            vertical_space(),
            sidebar_button(
                "Settings",
                SvgIcon::Settings,
                RouteName::Settings,
                keystache
            )
            .on_press(Message::Navigate(RouteName::Settings)),
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
