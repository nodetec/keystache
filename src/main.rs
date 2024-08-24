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
    type Message = Message;
    type Theme = Theme;
    type Flags = ();
    type Renderer = Renderer;

    fn new(_flags: Self::Flags) -> (Self, Command<Message>) {
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

    fn update(&mut self, event: Message) -> Command<Message> {
        self.page.update(event);

        Command::none()
    }

    fn view(&self) -> Element<Message> {
        let Self { page, .. } = self;

        let mut content: Element<Message> = Element::new(scrollable(
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
                            .send(Message::IncomingNip46Request(Arc::new((
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
enum Message {
    UnlockPasswordInputChanged(String),
    UnlockToggleSecureInput,
    UnlockPasswordSubmitted,
    DbDeleteAllData,
    GoToHomePage,
    GoToAddKeypairPage,
    GoToSettingsPage,
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

fn format_timestamp(timestamp: &u64) -> String {
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

pub fn truncate_text(input: &str, max_len: usize, center: bool) -> String {
    match center {
        // Center the ellipses around middle of the string.
        true => {
            if input.len() > max_len {
                format!(
                    "{}...{}",
                    &input[..(max_len / 2)],
                    &input[(input.len() - max_len / 2)..]
                )
            } else {
                input.to_string()
            }
        }
        false => {
            if input.len() > max_len {
                format!("{}...", &input[input.len() - max_len..])
            } else {
                input.to_string()
            }
        }
    }
}
