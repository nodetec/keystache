use iced::widget::{Column, Text};

use crate::{ConnectedState, KeystacheMessage};

use super::container;

#[derive(Clone)]
pub struct BitcoinWallet {
    pub connected_state: ConnectedState,
}

impl BitcoinWallet {
    pub fn view<'a>(&self) -> Column<'a, KeystacheMessage> {
        container("Wallet").push(Text::new("Work in progress! Check back later."))
    }
}
