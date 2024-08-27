use std::{collections::VecDeque, sync::Arc};

use iced::{
    widget::{checkbox, row, text_input, Column, Space},
    Pixels, Task,
};

use crate::{
    db::Database,
    ui_components::{icon_button, PaletteColor, SvgIcon},
    ConnectedState, KeystacheMessage,
};

use super::container;

#[derive(Debug, Clone)]
pub enum Message {
    PasswordInputChanged(String),
    ToggleSecureInput,
    PasswordSubmitted,
}

#[derive(Clone)]
pub struct Page {
    pub password: String,
    pub is_secure: bool,
    pub db_already_exists: bool,
}

impl Page {
    pub fn update(&mut self, msg: Message) -> Task<KeystacheMessage> {
        match msg {
            Message::PasswordInputChanged(new_password) => {
                self.password = new_password;

                Task::none()
            }
            Message::ToggleSecureInput => {
                self.is_secure = !self.is_secure;

                Task::none()
            }
            Message::PasswordSubmitted => Database::open_or_create_in_app_data_dir(&self.password)
                .map_or(Task::none(), |db| {
                    let db = Arc::new(db);
                    Task::done(KeystacheMessage::NavigateHomeAndSetConnectedState(
                        ConnectedState {
                            db,
                            in_flight_nip46_requests: VecDeque::new(),
                        },
                    ))
                }),
        }
    }

    pub fn view<'a>(&self) -> Column<'a, KeystacheMessage> {
        let Self {
            password,
            is_secure,
            db_already_exists,
        } = self;

        let text_input = text_input("Password", password)
            .on_input(|input| KeystacheMessage::UnlockPage(Message::PasswordInputChanged(input)))
            .padding(10)
            .size(30);

        let container_name = if *db_already_exists {
            "Enter Password"
        } else {
            "Choose a Password"
        };

        let description = if *db_already_exists {
            "Your Keystache database is password-encrypted. Enter your password to unlock it."
        } else {
            "Keystache will encrypt all of your data at rest. If you forget your password, your keys will be unrecoverable from Keystache. So make sure to backup your keys and keep your password somewhere safe."
        };

        let next_button_text = if *db_already_exists {
            "Unlock"
        } else {
            "Set Password"
        };

        let mut container = container(container_name)
            .push(description)
            .push(row![
                text_input.secure(*is_secure),
                Space::with_width(Pixels(20.0)),
                checkbox("Show password", !is_secure)
                    .on_toggle(|_| KeystacheMessage::UnlockPage(Message::ToggleSecureInput))
            ])
            .push(
                icon_button(next_button_text, SvgIcon::LockOpen, PaletteColor::Primary)
                    .on_press_maybe(
                        (!password.is_empty())
                            .then_some(KeystacheMessage::UnlockPage(Message::PasswordSubmitted)),
                    ),
            );

        if *db_already_exists {
            container = container.push(
                icon_button("Delete All Data", SvgIcon::Delete, PaletteColor::Danger)
                    .on_press(KeystacheMessage::DbDeleteAllData),
            );
        }

        container
    }
}
