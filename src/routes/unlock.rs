use std::{collections::VecDeque, sync::Arc};

use directories::ProjectDirs;
use iced::{
    widget::{checkbox, row, text_input, Column, Space},
    Pixels, Task,
};
use nostr_sdk::bitcoin::{bip32::Xpriv, Network};

use crate::{
    db::Database,
    ui_components::{icon_button, PaletteColor, SvgIcon},
    ConnectedState, KeystacheMessage, Wallet,
};

use super::{container, Loadable};

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

                    // TODO: Handle this unwrap. We should initialize
                    // project directories elsewhere and pass them in.
                    let project_dirs = ProjectDirs::from("co", "nodetec", "keystache")
                        .ok_or_else(|| {
                            anyhow::anyhow!("Could not determine Keystache project directories.")
                        })
                        .unwrap();

                    // TODO: CRITICAL: Remove this hardcoded key.
                    // TODO: Retrieve network from elsewhere rather than hardcoding.
                    let wallet = Arc::new(Wallet::new(
                        Xpriv::new_master(Network::Bitcoin, &[1, 2, 3, 4, 5, 6, 7, 8]).unwrap(),
                        Network::Bitcoin,
                        &project_dirs,
                    ));

                    // TODO: We should call `Task::chain()` and trigger a message rather than
                    // spawning a new task, since its completion doesn't trigger any UI event.
                    let wallet_clone = wallet.clone();
                    tokio::spawn(async move {
                        wallet_clone.connect_to_joined_federations().await.unwrap();
                    });

                    Task::done(KeystacheMessage::NavigateHomeAndSetConnectedState(
                        ConnectedState {
                            db,
                            wallet,
                            in_flight_nip46_requests: VecDeque::new(),
                            loadable_federation_views: Loadable::Loading,
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
