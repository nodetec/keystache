use fedimint_core::{config::FederationId, Amount};
use iced::{
    widget::{center, column, row, Column, Container, Space, Text},
    Task,
};

use crate::{
    app,
    fedimint::WalletView,
    ui_components::{icon_button, PaletteColor, SvgIcon, Toast, ToastStatus},
    util::format_amount,
};

use super::{ConnectedState, Loadable, RouteName};

mod federations;
mod receive;
mod send;

#[derive(Debug, Clone)]
pub enum Message {
    LeaveFederation(FederationId),
    LeftFederation(FederationId),

    Send(send::Message),
    Receive(receive::Message),
    Federations(federations::Message),

    UpdateWalletView(WalletView),
}

pub struct Page {
    pub connected_state: ConnectedState,
    pub subroute: Subroute,
}

impl Page {
    // TODO: Remove this clippy allow.
    #[allow(clippy::too_many_lines)]
    pub fn update(&mut self, msg: Message) -> Task<app::Message> {
        match msg {
            Message::LeaveFederation(federation_id) => {
                let wallet = self.connected_state.wallet.clone();

                Task::stream(async_stream::stream! {
                    match wallet.leave_federation(federation_id).await {
                        Ok(()) => {
                            yield app::Message::AddToast(Toast {
                                title: "Left federation".to_string(),
                                body: "You have successfully left the federation.".to_string(),
                                status: ToastStatus::Good,
                            });

                            yield app::Message::Routes(super::Message::BitcoinWalletPage(
                                Message::LeftFederation(federation_id)
                            ));
                        }
                        Err(err) => {
                            yield app::Message::AddToast(Toast {
                                title: "Failed to leave federation".to_string(),
                                body: format!("Failed to leave the federation: {err}"),
                                status: ToastStatus::Bad,
                            });
                        }
                    }
                })
            }
            Message::LeftFederation(federation_id) => {
                // A verbose way of saying "if the user is currently on the FederationDetails page and the federation ID matches the one that was just left, navigate back to the `Main` page".
                if let Subroute::Federations(federations) = &self.subroute {
                    if let federations::Subroute::FederationDetails(federation_details) =
                        &federations.subroute
                    {
                        if federation_details.federation_view().federation_id == federation_id {
                            return Task::done(app::Message::Routes(super::Message::Navigate(
                                RouteName::BitcoinWallet(SubrouteName::Main),
                            )));
                        }
                    }
                }

                Task::none()
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
            Message::Federations(federations_message) => {
                if let Subroute::Federations(federations_page) = &mut self.subroute {
                    federations_page.update(federations_message)
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
            Subroute::Main(main) => main.view(&self.connected_state),
            Subroute::Send(send) => send.view(),
            Subroute::Receive(receive) => receive.view(),
            Subroute::Federations(federations) => federations.view(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubrouteName {
    Main,
    Send,
    Receive,
    Federations(federations::SubrouteName),
}

impl SubrouteName {
    pub fn to_default_subroute(&self, connected_state: &ConnectedState) -> Subroute {
        match self {
            Self::Main => Subroute::Main(Main {}),
            Self::Send => Subroute::Send(send::Page::new(connected_state)),
            Self::Receive => Subroute::Receive(receive::Page::new(connected_state)),
            Self::Federations(subroute_name) => Subroute::Federations(federations::Page::new(
                connected_state,
                subroute_name.to_default_subroute(connected_state),
            )),
        }
    }
}

pub enum Subroute {
    Main(Main),
    Send(send::Page),
    Receive(receive::Page),
    Federations(federations::Page),
}

impl Subroute {
    pub fn to_name(&self) -> SubrouteName {
        match self {
            Self::Main(_) => SubrouteName::Main,
            Self::Send(_) => SubrouteName::Send,
            Self::Receive(_) => SubrouteName::Receive,
            Self::Federations(federations) => {
                SubrouteName::Federations(federations.subroute.to_name())
            }
        }
    }
}

pub struct Main {}

impl Main {
    // TODO: Remove this clippy allow.
    #[allow(clippy::unused_self)]
    fn view<'a>(&self, connected_state: &ConnectedState) -> Column<'a, app::Message> {
        let mut container = Column::new().spacing(20).align_x(iced::Alignment::Center);

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
                        .size(50),
                    )
                    .push(row![
                        icon_button("Send", SvgIcon::ArrowUpward, PaletteColor::Primary).on_press(
                            app::Message::Routes(super::Message::Navigate(
                                RouteName::BitcoinWallet(SubrouteName::Send)
                            ))
                        ),
                        Space::with_width(20.0),
                        icon_button("Receive", SvgIcon::ArrowDownward, PaletteColor::Primary)
                            .on_press(app::Message::Routes(super::Message::Navigate(
                                RouteName::BitcoinWallet(SubrouteName::Receive)
                            )))
                    ]);
            }
            Loadable::Failed => {
                container =
                    container.push(Text::new("Failed to load federation config views.").size(25));
            }
        }

        container = container.push(
            icon_button("View Federations", SvgIcon::Groups, PaletteColor::Primary).on_press(
                app::Message::Routes(super::Message::Navigate(RouteName::BitcoinWallet(
                    SubrouteName::Federations(federations::SubrouteName::List),
                ))),
            ),
        );

        column![center(Container::new(container))]
    }
}
