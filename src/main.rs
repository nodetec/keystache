#![deny(clippy::pedantic, clippy::nursery)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::match_same_arms)]
#![allow(clippy::missing_const_for_fn)]
#![allow(clippy::significant_drop_tightening)]

mod db;
mod routes;
mod ui_components;

use std::collections::VecDeque;
use std::sync::Arc;

use db::Database;

use iced::advanced::Application;
use iced::futures::{SinkExt, StreamExt};
use iced::widget::{column, container, row, scrollable, Theme};
use iced::window::settings::PlatformSpecific;
use iced::{Command, Element, Length, Pixels, Renderer, Settings, Size};
use nip_55::nip_46::{Nip46OverNip55ServerStream, Nip46RequestApproval};
use nostr_sdk::PublicKey;
use routes::{Route, RouteName};
use ui_components::sidebar;

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
                width: 800.0,
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
    page: Route,
}

impl Application for Keystache {
    type Executor = iced::executor::Default;
    type Message = KeystacheMessage;
    type Theme = Theme;
    type Flags = ();
    type Renderer = Renderer;

    fn new(_flags: Self::Flags) -> (Self, Command<KeystacheMessage>) {
        (
            Self {
                page: Route::new_locked(),
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        "Keystache".to_string()
    }

    fn update(&mut self, event: KeystacheMessage) -> Command<KeystacheMessage> {
        self.page.update(event);

        Command::none()
    }

    fn view(&self) -> Element<KeystacheMessage> {
        let Self { page, .. } = self;

        let mut content: Element<KeystacheMessage> = Element::new(scrollable(
            container(column![page.view()].spacing(20).padding(20))
                .width(Length::Fill)
                .center_x(),
        ));

        if page.to_name() != RouteName::Unlock {
            content = Element::new(row![sidebar(self), content]);
        };

        container(content).height(Length::Fill).center_y().into()
    }

    fn subscription(&self) -> iced::Subscription<Self::Message> {
        let Some(connected_state) = self.page.get_connected_state() else {
            return iced::Subscription::none();
        };

        let db_clone = connected_state.db.clone();

        iced::subscription::channel(
            std::any::TypeId::of::<Nip46OverNip55ServerStream>(),
            100,
            |mut output| async move {
                loop {
                    let mut stream = Nip46OverNip55ServerStream::start(
                        "/tmp/nip55-kind24133.sock",
                        db_clone.clone(),
                    )
                    .unwrap();

                    while let Some((request_list, public_key, response_sender)) =
                        stream.next().await
                    {
                        output
                            .send(KeystacheMessage::IncomingNip46Request(Arc::new((
                                request_list,
                                public_key,
                                response_sender,
                            ))))
                            .await
                            .unwrap();
                    }
                }
            },
        )
    }
}

#[derive(Debug, Clone)]
enum KeystacheMessage {
    Navigate(RouteName),
    UnlockPasswordInputChanged(String),
    UnlockToggleSecureInput,
    UnlockPasswordSubmitted,
    DbDeleteAllData,
    SaveKeypair,
    SaveKeypairNsecInputChanged(String),
    IncomingNip46Request(
        Arc<(
            Vec<nostr_sdk::nips::nip46::Request>,
            PublicKey,
            iced::futures::channel::oneshot::Sender<Nip46RequestApproval>,
        )>,
    ),
    ApproveFirstIncomingNip46Request,
    RejectFirstIncomingNip46Request,
}

#[derive(Clone)]
struct ConnectedState {
    db: Arc<Database>,
    #[allow(clippy::type_complexity)]
    in_flight_nip46_requests: VecDeque<
        Arc<(
            Vec<nostr_sdk::nips::nip46::Request>,
            PublicKey,
            iced::futures::channel::oneshot::Sender<Nip46RequestApproval>,
        )>,
    >,
}

fn format_timestamp(timestamp: u64) -> String {
    let signed = timestamp.to_owned() as i64;
    let date_time = chrono::DateTime::from_timestamp(signed, 0).unwrap();
    format!("{}", date_time.format("%m/%d/%Y, %l:%M %P"))
}

fn format_amount(amount: u64) -> String {
    if amount == 1 {
        return "1 sat".to_string();
    }

    let num = amount
        .to_string()
        .as_bytes()
        .rchunks(3)
        .rev()
        .map(std::str::from_utf8)
        .collect::<Result<Vec<&str>, _>>()
        .unwrap()
        .join(",");

    format!("{num} sats")
}

#[must_use]
pub fn truncate_text(input: &str, max_len: usize, center: bool) -> String {
    const ELLIPSES: &str = "...";
    const ELLIPSES_LEN: usize = ELLIPSES.len();

    let chars = input.chars().collect::<Vec<_>>();

    if chars.len() <= max_len {
        return input.to_string();
    }

    if max_len <= ELLIPSES_LEN {
        return ELLIPSES.to_string();
    }

    if center {
        // The number of total characters from `input` to display.
        // Subtract 3 for the ellipsis.
        let chars_to_display = max_len - 3;

        let is_lobsided = chars_to_display % 2 != 0;

        let chars_in_front = if is_lobsided {
            (chars_to_display / 2) + 1
        } else {
            chars_to_display / 2
        };

        let chars_in_back = chars_to_display / 2;

        format!(
            "{}{ELLIPSES}{}",
            &chars[..chars_in_front].iter().collect::<String>(),
            &chars[(chars.len() - chars_in_back)..]
                .iter()
                .collect::<String>()
        )
    } else {
        format!(
            "{}{ELLIPSES}",
            &chars[..(max_len - ELLIPSES_LEN)].iter().collect::<String>()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_text() {
        // Test short input (no truncation needed).
        assert_eq!(truncate_text("Hello", 10, false), "Hello");
        assert_eq!(truncate_text("Hello", 10, true), "Hello");

        // Test input exactly matching `max_len`.
        assert_eq!(truncate_text("Hello", 5, false), "Hello");
        assert_eq!(truncate_text("Hello", 5, true), "Hello");

        // Test long input.
        assert_eq!(truncate_text("Hello, world!", 8, false), "Hello...");
        assert_eq!(truncate_text("Hello, world!", 8, true), "Hel...d!");

        // Test Unicode string handling.
        assert_eq!(truncate_text("こんにちは世界", 6, false), "こんに...");
        assert_eq!(truncate_text("こんにちは世界", 6, true), "こん...界");

        // Test empty input.
        assert_eq!(truncate_text("", 5, false), "");
        assert_eq!(truncate_text("", 5, true), "");

        // Test edge cases with small `max_len` values.
        assert_eq!(truncate_text("Hello, world!", 0, false), "...");
        assert_eq!(truncate_text("Hello, world!", 0, true), "...");
        assert_eq!(truncate_text("Hello, world!", 1, false), "...");
        assert_eq!(truncate_text("Hello, world!", 1, true), "...");
        assert_eq!(truncate_text("Hello, world!", 2, false), "...");
        assert_eq!(truncate_text("Hello, world!", 2, true), "...");
        assert_eq!(truncate_text("Hello, world!", 3, false), "...");
        assert_eq!(truncate_text("Hello, world!", 3, true), "...");
        assert_eq!(truncate_text("Hello, world!", 4, false), "H...");
        assert_eq!(truncate_text("Hello, world!", 4, true), "H...");
        assert_eq!(truncate_text("Hello, world!", 5, false), "He...");
        assert_eq!(truncate_text("Hello, world!", 5, true), "H...!");
        assert_eq!(truncate_text("Hello, world!", 6, false), "Hel...");
        assert_eq!(truncate_text("Hello, world!", 6, true), "He...!");
        assert_eq!(truncate_text("Hello, world!", 7, false), "Hell...");
        assert_eq!(truncate_text("Hello, world!", 7, true), "He...d!");
    }
}
