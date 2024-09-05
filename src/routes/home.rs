use iced::widget::{Column, Text};

use crate::{ConnectedState, KeystacheMessage};

use super::container;

pub struct Page {
    pub connected_state: ConnectedState,
}

impl Page {
    // TODO: Remove this clippy allow.
    #[allow(clippy::unused_self)]
    pub fn view<'a>(&self) -> Column<'a, KeystacheMessage> {
        container("Home").push(Text::new("Work in progress! Check back later."))
    }
}
