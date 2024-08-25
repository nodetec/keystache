use iced::widget::{Column, Text};

use crate::{ConnectedState, KeystacheMessage};

use super::container;

#[derive(Clone)]
pub struct Page {
    pub connected_state: ConnectedState,
}

impl Page {
    pub fn view<'a>(&self) -> Column<'a, KeystacheMessage> {
        container("Settings").push(Text::new("Work in progress! Check back later."))
    }
}
