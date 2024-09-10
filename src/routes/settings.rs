use iced::{
    widget::{text_input, Column, Text},
    Task,
};

use crate::{
    ui_components::{icon_button, PaletteColor, SvgIcon},
    ConnectedState, KeystacheMessage,
};

use super::{container, RouteName};

// TODO: Remove this clippy allow once we have more variants.
#[allow(clippy::enum_variant_names)]
#[derive(Debug, Clone)]
pub enum Message {
    ChangePasswordCurrentPasswordInputChanged(String),
    ChangePasswordNewPasswordInputChanged(String),
    ChangePasswordNewPasswordConfirmationInputChanged(String),
    ChangePasswordSubmit {
        current_password: String,
        new_password: String,
    },
}

pub struct Page {
    pub connected_state: ConnectedState,
    pub subroute: Subroute,
}

impl Page {
    pub fn update(&mut self, msg: Message) -> Task<KeystacheMessage> {
        match msg {
            Message::ChangePasswordCurrentPasswordInputChanged(input) => {
                if let Subroute::ChangePassword(change_password) = &mut self.subroute {
                    change_password.current_password_input = input;
                }

                Task::none()
            }
            Message::ChangePasswordNewPasswordInputChanged(input) => {
                if let Subroute::ChangePassword(change_password) = &mut self.subroute {
                    change_password.new_password_input = input;
                }

                Task::none()
            }
            Message::ChangePasswordNewPasswordConfirmationInputChanged(input) => {
                if let Subroute::ChangePassword(change_password) = &mut self.subroute {
                    change_password.new_password_confirmation_input = input;
                }

                Task::none()
            }
            Message::ChangePasswordSubmit {
                current_password,
                new_password,
            } => {
                if self
                    .connected_state
                    .db
                    .change_password(&current_password, &new_password)
                    .is_ok()
                {
                    // TODO: Show success in UI.

                    Task::done(KeystacheMessage::Navigate(RouteName::Settings(
                        SubrouteName::Main,
                    )))
                } else {
                    // TODO: Show error in UI.
                    Task::none()
                }
            }
        }
    }

    pub fn view<'a>(&self) -> Column<'a, KeystacheMessage> {
        match &self.subroute {
            Subroute::Main(main) => main.view(),
            Subroute::ChangePassword(change_password) => change_password.view(),
            Subroute::About(about) => about.view(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubrouteName {
    Main,
    ChangePassword,
    About,
}

impl SubrouteName {
    pub fn to_default_subroute(&self) -> Subroute {
        match self {
            Self::Main => Subroute::Main(Main {}),
            Self::ChangePassword => Subroute::ChangePassword(ChangePassword {
                current_password_input: String::new(),
                new_password_input: String::new(),
                new_password_confirmation_input: String::new(),
            }),
            Self::About => Subroute::About(About {}),
        }
    }
}

pub enum Subroute {
    Main(Main),
    ChangePassword(ChangePassword),
    About(About),
}

impl Subroute {
    pub fn to_name(&self) -> SubrouteName {
        match self {
            Self::Main(_) => SubrouteName::Main,
            Self::ChangePassword(_) => SubrouteName::ChangePassword,
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
            .push(
                icon_button("Change Password", SvgIcon::Lock, PaletteColor::Primary).on_press(
                    KeystacheMessage::Navigate(RouteName::Settings(SubrouteName::ChangePassword)),
                ),
            )
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

// TODO: Remove this clippy allow.
#[allow(clippy::struct_field_names)]
pub struct ChangePassword {
    current_password_input: String,
    new_password_input: String,
    new_password_confirmation_input: String,
}

impl ChangePassword {
    // TODO: Remove this clippy allow.
    #[allow(clippy::unused_self)]
    fn view<'a>(&self) -> Column<'a, KeystacheMessage> {
        container("Change Password")
            .push(
                text_input("Current Password", &self.current_password_input)
                    .on_input(|input| {
                        KeystacheMessage::SettingsPage(
                            Message::ChangePasswordCurrentPasswordInputChanged(input),
                        )
                    })
                    .secure(true)
                    .padding(10)
                    .size(30),
            )
            .push(
                text_input("New Password", &self.new_password_input)
                    .on_input(|input| {
                        KeystacheMessage::SettingsPage(
                            Message::ChangePasswordNewPasswordInputChanged(input),
                        )
                    })
                    .secure(true)
                    .padding(10)
                    .size(30),
            )
            .push(
                text_input(
                    "Confirm New Password",
                    &self.new_password_confirmation_input,
                )
                .on_input(|input| {
                    KeystacheMessage::SettingsPage(
                        Message::ChangePasswordNewPasswordConfirmationInputChanged(input),
                    )
                })
                .secure(true)
                .padding(10)
                .size(30),
            )
            .push(
                icon_button("Change Password", SvgIcon::Lock, PaletteColor::Primary)
                    .on_press_maybe(
                        (!self.current_password_input.is_empty()
                            && !self.new_password_input.is_empty()
                            && self.new_password_input == self.new_password_confirmation_input)
                            .then(|| {
                                KeystacheMessage::SettingsPage(Message::ChangePasswordSubmit {
                                    current_password: self.current_password_input.clone(),
                                    new_password: self.new_password_input.clone(),
                                })
                            }),
                    ),
            )
            .push(
                icon_button("Back", SvgIcon::ArrowBack, PaletteColor::Background).on_press(
                    KeystacheMessage::Navigate(RouteName::Settings(SubrouteName::Main)),
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
