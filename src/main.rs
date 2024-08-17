#![deny(clippy::pedantic, clippy::nursery)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::match_same_arms)]
#![allow(clippy::missing_const_for_fn)]
#![allow(clippy::significant_drop_tightening)]

mod db;

use std::str::FromStr;

use db::Database;

use iced::widget::{
    checkbox, column, container, row, scrollable, text, text_input, Button, Column, Container,
    Space, Text,
};
use iced::window::settings::PlatformSpecific;
use iced::{Element, Length, Pixels, Sandbox, Settings, Size};
use nostr_sdk::secp256k1::{Keypair, Secp256k1};
use nostr_sdk::SecretKey;

fn main() -> iced::Result {
    tracing_subscriber::fmt::init();

    Keystache::run(Settings {
        id: None,
        window: iced::window::Settings {
            size: iced::Size {
                width: 470.0,
                height: 620.0,
            },
            position: iced::window::Position::Default,
            min_size: Some(Size {
                width: 400.0,
                height: 600.0,
            }),
            max_size: None,
            visible: true,
            resizable: true,
            decorations: true,
            transparent: false,
            level: iced::window::Level::Normal,
            icon: None,                                     // TODO: Set icon.
            platform_specific: PlatformSpecific::default(), // TODO: Set platform specific settings for each platform.
            exit_on_close_request: true,
        },
        flags: (),
        fonts: Vec::new(),
        default_font: iced::Font::default(),
        default_text_size: Pixels(16.0),
        antialiasing: false,
    })
}

struct Keystache {
    page: Page,
}

impl Sandbox for Keystache {
    type Message = Message;

    fn new() -> Self {
        Self {
            page: Page::DbUnlock {
                password: String::new(),
                is_secure: true,
                db_already_exists: Database::exists(),
            },
        }
    }

    fn title(&self) -> String {
        "Keystache".to_string()
    }

    fn update(&mut self, event: Message) {
        self.page.update(event);
    }

    fn view(&self) -> Element<Message> {
        let Self { page, .. } = self;

        let content: Element<_> = column![page.view()].spacing(20).padding(20).into();

        let scrollable = scrollable(container(content).width(Length::Fill).center_x());

        container(scrollable).height(Length::Fill).center_y().into()
    }
}

#[derive(Debug, Clone)]
enum Message {
    DbUnlockPasswordInputChanged(String),
    DbUnlockToggleSecureInput,
    DbUnlockPasswordSubmitted,
    DbDeleteAllData,
    GoToHomePage,
    GoToAddKeypairPage,
    SaveKeypair,
    SaveKeypairNsecInputChanged(String),
}

enum Page {
    DbUnlock {
        password: String,
        is_secure: bool,
        db_already_exists: bool,
    },
    Home {
        db: Database,
    },
    AddKeypair {
        nsec: String,
        keypair_or: Option<Keypair>, // Parsed from nsec on any update. `Some` if nsec is valid, `None` otherwise.
        db: Database,
    },
}

impl<'a> Page {
    fn update(&mut self, msg: Message) {
        match msg {
            Message::DbUnlockPasswordInputChanged(new_password) => {
                if let Self::DbUnlock { password, .. } = self {
                    *password = new_password;
                }
            }
            Message::DbUnlockToggleSecureInput => {
                if let Self::DbUnlock { is_secure, .. } = self {
                    *is_secure = !*is_secure;
                }
            }
            Message::DbUnlockPasswordSubmitted => {
                if let Self::DbUnlock { password, .. } = self {
                    if let Ok(db) = Database::open_or_create_in_app_data_dir(password) {
                        *self = Self::Home { db };
                    }
                }
            }
            Message::DbDeleteAllData => {
                if let Self::DbUnlock {
                    db_already_exists, ..
                } = self
                {
                    Database::delete();
                    *db_already_exists = false;
                }
            }
            Message::GoToHomePage => {
                if let Some(db) = self.get_db() {
                    *self = Self::Home { db: db.clone() };
                }
            }
            Message::GoToAddKeypairPage => {
                if let Self::Home { db } = self {
                    *self = Self::AddKeypair {
                        nsec: String::new(),
                        keypair_or: None,
                        db: db.clone(),
                    };
                }
            }
            Message::SaveKeypair => {
                if let Self::AddKeypair {
                    keypair_or: Some(keypair),
                    db,
                    ..
                } = self
                {
                    db.save_keypair(keypair);
                }
            }
            Message::SaveKeypairNsecInputChanged(new_nsec) => {
                if let Self::AddKeypair {
                    nsec, keypair_or, ..
                } = self
                {
                    *nsec = new_nsec;

                    // Set `keypair_or` to `Some` if `nsec` is a valid secret key, `None` otherwise.
                    *keypair_or = SecretKey::from_str(nsec).map_or(None, |secret_key| {
                        Some(Keypair::from_secret_key(&Secp256k1::new(), &secret_key))
                    });
                }
            }
        };
    }

    fn view(&self) -> Element<Message> {
        match self {
            Self::DbUnlock {
                password,
                is_secure,
                db_already_exists,
            } => Self::db_unlock(password, *is_secure, *db_already_exists),
            Self::Home { db } => Self::home(db),
            Self::AddKeypair {
                nsec,
                keypair_or,
                db,
            } => Self::add_keypair(nsec, keypair_or, db),
        }
        .into()
    }

    fn container(title: &str) -> Column<'a, Message> {
        column![text(title).size(35)]
            .spacing(20)
            .align_items(iced::Alignment::Center)
    }

    fn get_db(&self) -> Option<&Database> {
        match self {
            Self::DbUnlock { .. } => None,
            Self::Home { db } => Some(db),
            Self::AddKeypair { db, .. } => Some(db),
        }
    }

    fn db_unlock(password: &str, is_secure: bool, db_already_exists: bool) -> Column<'a, Message> {
        let text_input = text_input("Password", password)
            .on_input(Message::DbUnlockPasswordInputChanged)
            .padding(10)
            .size(30);

        let container_name = if db_already_exists {
            "Enter Password"
        } else {
            "Choose a Password"
        };

        let description = if db_already_exists {
            "Your Keystache database is password-encrypted. Enter your password to unlock it."
        } else {
            "Keystache will encrypt all of your data at rest. If you forget your password, your keys will be unrecoverable from Keystache. So make sure to backup your keys and keep your password somewhere safe."
        };

        let next_button_text = if db_already_exists {
            "Unlock"
        } else {
            "Set Password"
        };

        let mut container = Self::container(container_name)
            .push(description)
            .push(row![
                text_input.secure(is_secure),
                Space::with_width(Pixels(20.0)),
                checkbox("Show password", !is_secure)
                    .on_toggle(|_| Message::DbUnlockToggleSecureInput)
            ])
            .push(
                Button::new(
                    Container::new(
                        Text::new(next_button_text)
                            .horizontal_alignment(iced::alignment::Horizontal::Center),
                    )
                    .width(Length::Fill)
                    .center_x(),
                )
                .padding([12, 24])
                .on_press_maybe(
                    (!password.is_empty()).then_some(Message::DbUnlockPasswordSubmitted),
                ),
            );

        if db_already_exists {
            container = container.push(
                Button::new(
                    Container::new(
                        Text::new("Delete All Data")
                            .horizontal_alignment(iced::alignment::Horizontal::Center),
                    )
                    .width(Length::Fill)
                    .center_x(),
                )
                .padding([12, 24])
                .on_press(Message::DbDeleteAllData),
            );
        }

        container
    }

    fn home(db: &Database) -> Column<'a, Message> {
        // TODO: Add pagination.
        let Ok(public_keys) = db.list_public_keys(999, 0) else {
            return Self::container("Desktop companion for Nostr apps").push("Failed to load keys");
        };

        let mut container =
            Self::container("Desktop companion for Nostr apps").push("Manage your Nostr accounts");

        for public_key in public_keys {
            container = container.push(
                Text::new(public_key)
                    .size(20)
                    .horizontal_alignment(iced::alignment::Horizontal::Center),
            );
        }

        container = container.push(
            Button::new(
                Container::new(
                    Text::new("Add Keypair")
                        .horizontal_alignment(iced::alignment::Horizontal::Center),
                )
                .width(Length::Fill)
                .center_x(),
            )
            .padding([12, 24])
            .on_press(Message::GoToAddKeypairPage),
        );

        container
    }

    fn add_keypair(nsec: &str, keypair_or: &Option<Keypair>, db: &Database) -> Column<'a, Message> {
        Self::container("Add Keypair")
            .push(
                text_input("nSec", nsec)
                    .on_input(Message::SaveKeypairNsecInputChanged)
                    .padding(10)
                    .size(30),
            )
            .push(
                Button::new(
                    Container::new(
                        Text::new("Save").horizontal_alignment(iced::alignment::Horizontal::Center),
                    )
                    .width(Length::Fill)
                    .center_x(),
                )
                .padding([12, 24])
                .on_press_maybe(keypair_or.is_some().then_some(Message::SaveKeypair)),
            )
            .push(
                Button::new(
                    Container::new(
                        Text::new("Back").horizontal_alignment(iced::alignment::Horizontal::Center),
                    )
                    .width(Length::Fill)
                    .center_x(),
                )
                .padding([12, 24])
                .on_press(Message::GoToHomePage),
            )
    }
}
