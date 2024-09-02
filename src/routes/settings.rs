use iced::{
    widget::{Column, Text},
    Task,
};

use crate::{
    ui_components::{icon_button, PaletteColor, SvgIcon},
    ConnectedState, KeystacheMessage,
};

use super::{container, RouteName};

#[derive(Debug, Clone)]
pub enum Message {}

pub struct Page {
    pub connected_state: ConnectedState,
    pub subroute: Subroute,
}

impl Page {
    pub fn update(&mut self, msg: Message) -> Task<KeystacheMessage> {
        match msg {}
    }

    pub fn view<'a>(&self) -> Column<'a, KeystacheMessage> {
        match &self.subroute {
            Subroute::Main(main) => main.view(),
            Subroute::About(about) => about.view(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubrouteName {
    Main,
    About,
}

impl SubrouteName {
    pub fn to_default_subroute(&self) -> Subroute {
        match self {
            Self::Main => Subroute::Main(Main {}),
            Self::About => Subroute::About(About {}),
        }
    }
}

pub enum Subroute {
    Main(Main),
    About(About),
}

impl Subroute {
    pub fn to_name(&self) -> SubrouteName {
        match self {
            Self::Main(_) => SubrouteName::Main,
            Self::About(_) => SubrouteName::About,
        }
    }
}

pub struct Main {}

impl Main {
    // TODO: Remove this clippy allow.
    #[allow(clippy::unused_self)]
    fn view<'a>(&self) -> Column<'a, KeystacheMessage> {
        container("Settings")
            .push(icon_button(
                "Change Password (Coming Soon)",
                SvgIcon::Lock,
                PaletteColor::Primary,
            ))
            .push(icon_button(
                "Backup (Coming Soon)",
                SvgIcon::FileCopy,
                PaletteColor::Primary,
            ))
            .push(
                icon_button("About", SvgIcon::Info, PaletteColor::Primary).on_press(
                    KeystacheMessage::Navigate(RouteName::Settings(SubrouteName::About)),
                ),
            )
    }
}

pub struct About {}

impl About {
    // TODO: Remove this clippy allow.
    #[allow(clippy::unused_self)]
    fn view<'a>(&self) -> Column<'a, KeystacheMessage> {
        container("About")
            .push(Text::new("Description").size(25))
            .push(Text::new("Keystache is a Nostr single-sign-on key management and Fedimint Bitcoin wallet created by Tommy Volk and generously funded by OpenSats").size(15))
            .push(Text::new("Source Code").size(25))
            .push(Text::new("https://github.com/Open-Source-Justice-Foundation/Keystache").size(15))
            .push(Text::new("Version").size(25))
            .push(Text::new(env!("CARGO_PKG_VERSION")).size(15))
            .push(icon_button("Back", SvgIcon::ArrowBack, PaletteColor::Background).on_press(
                KeystacheMessage::Navigate(RouteName::Settings(SubrouteName::Main))
            ))
    }
}
