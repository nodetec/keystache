use iced::widget::{Column, Text};

use crate::{
    truncate_text,
    ui_components::{icon_button, PaletteColor, SvgIcon},
    ConnectedState, Message,
};

use super::container;

#[derive(Clone)]
pub struct Home {
    pub connected_state: ConnectedState,
}

impl Home {
    pub fn view<'a>(&self) -> Column<'a, Message> {
        // TODO: Add pagination.
        let Ok(public_keys) = self.connected_state.db.list_public_keys(999, 0) else {
            return container("Desktop companion for Nostr apps").push("Failed to load keys");
        };

        let mut container =
            container("Desktop companion for Nostr apps").push("Manage your Nostr accounts");

        for public_key in public_keys {
            container = container.push(
                Text::new(truncate_text(&public_key, 12, true))
                    .size(20)
                    .horizontal_alignment(iced::alignment::Horizontal::Center),
            );
        }

        container = container.push(
            icon_button("Add Keypair", SvgIcon::Key, PaletteColor::Primary)
                .on_press(Message::GoToAddKeypairPage),
        );

        container
    }
}
