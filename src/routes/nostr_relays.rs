use iced::widget::{Column, Text};

use crate::{ConnectedState, Message};

use super::container;

#[derive(Clone)]
pub struct NostrRelays {
    pub connected_state: ConnectedState,
}

impl NostrRelays {
    pub fn view<'a>(&self) -> Column<'a, Message> {
        container("Nostr Relays").push(Text::new("Work in progress! Check back later."))
    }
}
