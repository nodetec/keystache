use iced::widget::{Column, Text};

use crate::{util::format_amount_sats, ConnectedState, KeystacheMessage};

use super::container;

#[derive(Clone)]
pub struct Page {
    pub connected_state: ConnectedState,
}

impl Page {
    // TODO: Remove this clippy allow.
    #[allow(clippy::unused_self)]
    pub fn view<'a>(&self) -> Column<'a, KeystacheMessage> {
        container("Wallet")
            .push(Text::new("Work in progress! Check back later."))
            .push(Text::new(format!("Balance: {}", format_amount_sats(0))))
    }
}
