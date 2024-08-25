use iced::widget::{Column, Text};

use crate::{util::truncate_text, ConnectedState, KeystacheMessage};

use super::container;

#[derive(Clone)]
pub struct Page {
    pub connected_state: ConnectedState,
}

impl Page {
    pub fn view<'a>(&self) -> Column<'a, KeystacheMessage> {
        container("Home").push(Text::new("Work in progress! Check back later."))
    }
}
