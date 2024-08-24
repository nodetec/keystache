use iced::{
    widget::{Column, Text},
    Element,
};

use crate::{ConnectedState, Message};

use super::container;

#[derive(Clone)]
pub struct Settings {
    pub connected_state: ConnectedState,
}

impl Settings {
    pub fn view<'a>(&self) -> Column<'a, Message> {
        container("Settings").push(Text::new("Work in progress! Check back later."))
    }
}
