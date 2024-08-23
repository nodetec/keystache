use iced::widget::{text_input, Column};
use nostr_sdk::secp256k1::Keypair;

use crate::{
    ui_components::{icon_button, PaletteColor, SvgIcon},
    ConnectedState, Message,
};

use super::container;

#[derive(Clone)]
pub struct AddNostrKeypair {
    pub connected_state: ConnectedState,
    pub nsec: String,
    pub keypair_or: Option<Keypair>, // Parsed from nsec on any update. `Some` if nsec is valid, `None` otherwise.
}

impl AddNostrKeypair {
    pub fn view<'a>(&self) -> Column<'a, Message> {
        container("Add Keypair")
            .push(
                text_input("nSec", &self.nsec)
                    .on_input(Message::SaveKeypairNsecInputChanged)
                    .padding(10)
                    .size(30),
            )
            .push(
                icon_button("Save", SvgIcon::Save, PaletteColor::Primary)
                    .on_press_maybe(self.keypair_or.is_some().then_some(Message::SaveKeypair)),
            )
    }
}
