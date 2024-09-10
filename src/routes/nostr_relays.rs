use std::str::FromStr;

use iced::{
    widget::{row, text_input, Column, Text},
    Color, Task,
};
use nostr_relay_pool::RelayStatus;
use nostr_sdk::Url;

use crate::{
    app,
    nostr::NostrModuleMessage,
    ui_components::{icon_button, PaletteColor, SvgIcon, Toast, ToastStatus},
    util::truncate_text,
};

use super::{container, ConnectedState, RouteName};

#[derive(Debug, Clone)]
pub enum Message {
    SaveRelay { websocket_url: String },
    SaveRelayWebsocketUrlInputChanged(String),
    DeleteRelay { websocket_url: String },
}

pub struct Page {
    pub connected_state: ConnectedState,
    pub subroute: Subroute,
}

impl Page {
    pub fn update(&mut self, msg: Message) -> Task<app::Message> {
        match msg {
            Message::SaveRelay { websocket_url } => {
                let task = match self.connected_state.db.save_relay(websocket_url.clone()) {
                    Ok(()) => Task::done(app::Message::AddToast(Toast {
                        title: "Saved relay".to_string(),
                        body: "The relay was successfully saved.".to_string(),
                        status: ToastStatus::Good,
                    })),
                    Err(_err) => Task::done(app::Message::AddToast(Toast {
                        title: "Failed to save relay".to_string(),
                        body: "The relay was not saved.".to_string(),
                        status: ToastStatus::Bad,
                    })),
                };

                self.connected_state
                    .nostr_module
                    .update(NostrModuleMessage::ConnectToRelay(websocket_url));

                task
            }
            Message::SaveRelayWebsocketUrlInputChanged(new_websocket_url) => {
                if let Subroute::Add(Add { websocket_url }) = &mut self.subroute {
                    *websocket_url = new_websocket_url;
                }

                Task::none()
            }
            Message::DeleteRelay { websocket_url } => {
                let task = match self.connected_state.db.remove_relay(&websocket_url) {
                    Ok(()) => Task::done(app::Message::AddToast(Toast {
                        title: "Deleted relay".to_string(),
                        body: "The relay was successfully deleted.".to_string(),
                        status: ToastStatus::Good,
                    })),
                    Err(_err) => Task::done(app::Message::AddToast(Toast {
                        title: "Failed to delete relay".to_string(),
                        body: "The relay was not deleted.".to_string(),
                        status: ToastStatus::Bad,
                    })),
                };

                self.connected_state
                    .nostr_module
                    .update(NostrModuleMessage::DisconnectFromRelay(websocket_url));

                task
            }
        }
    }

    pub fn view<'a>(&self) -> Column<'a, app::Message> {
        match &self.subroute {
            Subroute::List(list) => list.view(&self.connected_state),
            Subroute::Add(add) => add.view(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubrouteName {
    List,
    Add,
}

impl SubrouteName {
    pub fn to_default_subroute(&self) -> Subroute {
        match self {
            Self::List => Subroute::List(List {}),
            Self::Add => Subroute::Add(Add {
                websocket_url: String::new(),
            }),
        }
    }
}

pub enum Subroute {
    List(List),
    Add(Add),
}

impl Subroute {
    pub fn to_name(&self) -> SubrouteName {
        match self {
            Self::List(_) => SubrouteName::List,
            Self::Add(_) => SubrouteName::Add,
        }
    }
}

pub struct List {}

impl List {
    // TODO: Remove this clippy allow.
    #[allow(clippy::unused_self)]
    fn view<'a>(&self, connected_state: &ConnectedState) -> Column<'a, app::Message> {
        // TODO: Add pagination.
        let Ok(relays) = connected_state.db.list_relays(999, 0) else {
            return container("Relays").push("Failed to load relays");
        };

        let mut container = container("Relays");

        for relay in relays {
            let relay_state_or = Url::from_str(&relay.websocket_url).map_or(None, |url| {
                connected_state.nostr_state.relay_connections.get(&url)
            });

            let relay_connection_color = relay_state_or.map_or_else(
                || Color::from_rgb(0.3, 0.3, 0.3),
                |relay_status| match relay_status {
                    RelayStatus::Initialized => Color::from_rgb(0.3, 0.3, 0.3),
                    RelayStatus::Pending | RelayStatus::Connecting => {
                        Color::from_rgb(0.8, 0.8, 0.0)
                    }
                    RelayStatus::Connected => Color::from_rgb(0.0, 0.8, 0.0),
                    RelayStatus::Disconnected | RelayStatus::Terminated => {
                        Color::from_rgb(0.8, 0.0, 0.0)
                    }
                },
            );

            container = container.push(row![
                Text::new(truncate_text(&relay.websocket_url, 12, true))
                    .size(20)
                    .align_x(iced::alignment::Horizontal::Center),
                icon_button("Delete", SvgIcon::Delete, PaletteColor::Danger).on_press(
                    app::Message::Routes(super::Message::NostrRelaysPage(Message::DeleteRelay {
                        websocket_url: relay.websocket_url
                    }))
                ),
                SvgIcon::Circle.view(24.0, 24.0, relay_connection_color),
            ]);
        }

        container = container.push(
            icon_button("Add Relay", SvgIcon::Add, PaletteColor::Primary).on_press(
                app::Message::Routes(super::Message::Navigate(RouteName::NostrRelays(
                    SubrouteName::Add,
                ))),
            ),
        );

        container
    }
}

pub struct Add {
    websocket_url: String,
}

impl Add {
    fn view<'a>(&self) -> Column<'a, app::Message> {
        container("Add Relay")
            .push(
                text_input("Websocket URL", &self.websocket_url)
                    .on_input(|input| {
                        app::Message::Routes(super::Message::NostrRelaysPage(
                            Message::SaveRelayWebsocketUrlInputChanged(input),
                        ))
                    })
                    .padding(10)
                    .size(30),
            )
            .push(
                icon_button("Save", SvgIcon::Save, PaletteColor::Primary).on_press(
                    app::Message::Routes(super::Message::NostrRelaysPage(Message::SaveRelay {
                        websocket_url: self.websocket_url.clone(),
                    })),
                ),
            )
            .push(
                icon_button("Back", SvgIcon::ArrowBack, PaletteColor::Background).on_press(
                    app::Message::Routes(super::Message::Navigate(RouteName::NostrRelays(
                        SubrouteName::List,
                    ))),
                ),
            )
    }
}
