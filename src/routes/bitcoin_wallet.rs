use iced::{
    widget::{Column, Text},
    Element,
};

use crate::{ConnectedState, Message};

use super::container;

#[derive(Clone)]
pub struct BitcoinWallet {
    pub connected_state: ConnectedState,
}

impl BitcoinWallet {
    pub fn view<'a>(&self) -> Column<'a, Message> {
        container("Wallet").push(Text::new("Work in progress! Check back later."))
    }
}
