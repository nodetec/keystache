#![deny(clippy::pedantic, clippy::nursery)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::match_same_arms)]
#![allow(clippy::missing_const_for_fn)]

mod db;

use db::Database;

use iced::widget::{
    checkbox, column, container, row, scrollable, text, text_input, Button, Column, Container,
    Space, Text,
};
use iced::window::settings::PlatformSpecific;
use iced::{Element, Length, Pixels, Sandbox, Settings, Size};

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

        let content: Element<_> = column![page.view()]
            .max_width(540)
            .spacing(20)
            .padding(20)
            .into();

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
}

enum Page {
    DbUnlock {
        password: String,
        is_secure: bool,
        db_already_exists: bool,
    },
    KeysList {
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
                    if let Ok(db) =
                        Database::open_or_create_in_app_data_dir((*password).to_string())
                    {
                        *self = Self::KeysList { db };
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
        };
    }

    fn view(&self) -> Element<Message> {
        match self {
            Self::DbUnlock {
                password,
                is_secure,
                db_already_exists,
            } => Self::db_unlock(password, *is_secure, *db_already_exists),
            Self::KeysList { .. } => Self::keys_list(),
        }
        .into()
    }

    fn container(title: &str) -> Column<'a, Message> {
        column![text(title).size(50)]
            .spacing(20)
            .align_items(iced::Alignment::Center)
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

    fn keys_list() -> Column<'a, Message> {
        Self::container("Keys").push("Here are your keys!")
    }
}
