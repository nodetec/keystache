use fedimint_core::Amount;
use iced::{
    widget::{container::Style, horizontal_space, row, Column, Container, Space, Text},
    Border, Length, Shadow, Task, Theme,
};

use crate::{
    app,
    fedimint::{FederationView, WalletView},
    ui_components::{icon_button, PaletteColor, SvgIcon},
    util::{format_amount, lighten},
};

use super::{container, ConnectedState, Loadable, RouteName};

mod add;
mod federation_details;
mod receive;
mod send;

#[derive(Debug, Clone)]
pub enum Message {
    FederationDetails(federation_details::Message),
    Add(add::Message),
    Send(send::Message),
    Receive(receive::Message),

    UpdateWalletView(WalletView),
}

pub struct Page {
    pub connected_state: ConnectedState,
    pub subroute: Subroute,
}

impl Page {
    pub fn update(&mut self, msg: Message) -> Task<app::Message> {
        match msg {
            Message::FederationDetails(federation_details_message) => {
                if let Subroute::FederationDetails(federation_details_page) = &mut self.subroute {
                    federation_details_page.update(federation_details_message)
                } else {
                    Task::none()
                }
            }
            Message::Add(add_message) => {
                if let Subroute::Add(add_page) = &mut self.subroute {
                    add_page.update(add_message)
                } else {
                    Task::none()
                }
            }
            Message::Send(send_message) => {
                if let Subroute::Send(send_page) = &mut self.subroute {
                    send_page.update(send_message)
                } else {
                    Task::none()
                }
            }
            Message::Receive(receive_message) => {
                if let Subroute::Receive(receive_page) = &mut self.subroute {
                    receive_page.update(receive_message)
                } else {
                    Task::none()
                }
            }
            Message::UpdateWalletView(wallet_view) => match &mut self.subroute {
                Subroute::Send(send_page) => {
                    send_page.update(send::Message::UpdateWalletView(wallet_view))
                }
                Subroute::Receive(receive_page) => {
                    receive_page.update(receive::Message::UpdateWalletView(wallet_view))
                }
                _ => Task::none(),
            },
        }
    }

    pub fn view(&self) -> Column<app::Message> {
        match &self.subroute {
            Subroute::List(list) => list.view(&self.connected_state),
            Subroute::FederationDetails(federation_details) => federation_details.view(),
            Subroute::Add(add) => add.view(),
            Subroute::Send(send) => send.view(),
            Subroute::Receive(receive) => receive.view(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubrouteName {
    List,
    FederationDetails(FederationView),
    Add,
    Send,
    Receive,
}

impl SubrouteName {
    pub fn to_default_subroute(&self, connected_state: &ConnectedState) -> Subroute {
        match self {
            Self::List => Subroute::List(List {}),
            Self::FederationDetails(federation_view) => Subroute::FederationDetails(
                federation_details::Page::new(federation_view.clone(), connected_state),
            ),
            Self::Add => Subroute::Add(add::Page::new(connected_state)),
            Self::Send => Subroute::Send(send::Page::new(connected_state)),
            Self::Receive => Subroute::Receive(receive::Page::new(connected_state)),
        }
    }
}

pub enum Subroute {
    List(List),
    FederationDetails(federation_details::Page),
    Add(add::Page),
    Send(send::Page),
    Receive(receive::Page),
}

impl Subroute {
    pub fn to_name(&self) -> SubrouteName {
        match self {
            Self::List(_) => SubrouteName::List,
            Self::FederationDetails(federation_details) => {
                SubrouteName::FederationDetails(federation_details.clone_view())
            }
            Self::Add(_) => SubrouteName::Add,
            Self::Send(_) => SubrouteName::Send,
            Self::Receive(_) => SubrouteName::Receive,
        }
    }
}

pub struct List {}

impl List {
    // TODO: Remove this clippy allow.
    #[allow(clippy::unused_self)]
    fn view<'a>(&self, connected_state: &ConnectedState) -> Column<'a, app::Message> {
        let mut container = container("Wallet");

        match &connected_state.loadable_wallet_view {
            Loadable::Loading => {
                container = container.push(Text::new("Loading federations...").size(25));
            }
            Loadable::Loaded(wallet_view) => {
                container = container
                    .push(
                        Text::new(format_amount(Amount::from_msats(
                            wallet_view
                                .federations
                                .values()
                                .map(|view| view.balance.msats)
                                .sum::<u64>(),
                        )))
                        .size(35),
                    )
                    .push(row![
                        icon_button("Send", SvgIcon::ArrowUpward, PaletteColor::Primary).on_press(
                            app::Message::Routes(super::Message::Navigate(
                                RouteName::BitcoinWallet(SubrouteName::Send)
                            ))
                        ),
                        Space::with_width(10.0),
                        icon_button("Receive", SvgIcon::ArrowDownward, PaletteColor::Primary)
                            .on_press(app::Message::Routes(super::Message::Navigate(
                                RouteName::BitcoinWallet(SubrouteName::Receive)
                            )))
                    ])
                    .push(Text::new("Federations").size(25));

                for view in wallet_view.federations.values() {
                    let column: Column<_, Theme, _> = Column::new()
                        .push(
                            Text::new(
                                view.name_or
                                    .clone()
                                    .unwrap_or_else(|| "Unnamed Federation".to_string()),
                            )
                            .size(25),
                        )
                        .push(Text::new(format_amount(view.balance)));

                    container = container.push(
                        Container::new(row![
                            column,
                            horizontal_space(),
                            icon_button("Details", SvgIcon::ChevronRight, PaletteColor::Background)
                                .on_press(app::Message::Routes(super::Message::Navigate(
                                    RouteName::BitcoinWallet(SubrouteName::FederationDetails(
                                        view.clone()
                                    ))
                                )))
                        ])
                        .padding(10)
                        .width(Length::Fill)
                        .style(|theme| -> Style {
                            Style {
                                text_color: None,
                                background: Some(lighten(theme.palette().background, 0.05).into()),
                                border: Border {
                                    color: iced::Color::WHITE,
                                    width: 0.0,
                                    radius: (8.0).into(),
                                },
                                shadow: Shadow::default(),
                            }
                        }),
                    );
                }
            }
            Loadable::Failed => {
                container =
                    container.push(Text::new("Failed to load federation config views.").size(25));
            }
        }

        container = container.push(
            icon_button("Join Federation", SvgIcon::Add, PaletteColor::Primary).on_press(
                app::Message::Routes(super::Message::Navigate(RouteName::BitcoinWallet(
                    SubrouteName::Add,
                ))),
            ),
        );

        container
    }
}
