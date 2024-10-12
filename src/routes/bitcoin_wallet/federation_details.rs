use std::sync::Arc;

use fedimint_core::config::FederationId;
use iced::{
    widget::{column, container::Style, Column, Container, Text},
    Border, Length, Shadow, Task, Theme,
};

use crate::{
    app,
    fedimint::{FederationView, Wallet},
    routes::{self, container, ConnectedState},
    ui_components::{icon_button, PaletteColor, SvgIcon, Toast, ToastStatus},
    util::{format_amount, lighten, truncate_text},
};

#[derive(Debug, Clone)]
pub enum Message {
    LeaveFederation(FederationId),
    LeftFederation(FederationId),
}

pub struct Page {
    wallet: Arc<Wallet>,
    view: FederationView,
}

impl Page {
    pub fn new(view: FederationView, connected_state: &ConnectedState) -> Self {
        Self {
            wallet: connected_state.wallet.clone(),
            view,
        }
    }

    pub fn clone_view(&self) -> FederationView {
        self.view.clone()
    }

    // TODO: Remove these clippy allows.
    #[allow(clippy::needless_pass_by_value)]
    #[allow(clippy::needless_pass_by_ref_mut)]
    pub fn update(&mut self, msg: Message) -> Task<app::Message> {
        match msg {
            Message::LeaveFederation(federation_id) => {
                let wallet = self.wallet.clone();

                Task::stream(async_stream::stream! {
                    match wallet.leave_federation(federation_id).await {
                        Ok(()) => {
                            yield app::Message::AddToast(Toast {
                                title: "Left federation".to_string(),
                                body: "You have successfully left the federation.".to_string(),
                                status: ToastStatus::Good,
                            });

                            yield app::Message::Routes(routes::Message::BitcoinWalletPage(
                                super::Message::FederationDetails(Message::LeftFederation(federation_id))
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
                // A verbose way of saying "if the user is currently on the FederationDetails page and the federation ID matches the one that was just left, navigate back to the List page".
                if self.view.federation_id == federation_id {
                    return Task::done(app::Message::Routes(routes::Message::Navigate(
                        routes::RouteName::BitcoinWallet(super::SubrouteName::List),
                    )));
                }

                Task::none()
            }
        }
    }

    pub fn view<'a>(&self) -> Column<'a, app::Message> {
        let mut container = container("Federation Details")
            .push(
                Text::new(
                    self.view
                        .name_or
                        .clone()
                        .unwrap_or_else(|| "Unnamed Federation".to_string()),
                )
                .size(25),
            )
            .push(Text::new(format!(
                "Federation ID: {}",
                truncate_text(&self.view.federation_id.to_string(), 23, true)
            )))
            .push(Text::new(format_amount(self.view.balance)))
            .push(Text::new("Gateways").size(20));

        for gateway in &self.view.gateways {
            let vetted_text = if gateway.vetted {
                "Vetted"
            } else {
                "Not Vetted"
            };

            let column: Column<_, Theme, _> = column![
                Text::new(format!(
                    "Gateway ID: {}",
                    truncate_text(&gateway.info.gateway_id.to_string(), 43, true)
                )),
                Text::new(format!(
                    "Lightning Node Alias: {}",
                    truncate_text(&gateway.info.lightning_alias.to_string(), 43, true)
                )),
                Text::new(format!(
                    "Lightning Node Public Key: {}",
                    truncate_text(&gateway.info.node_pub_key.to_string(), 43, true)
                )),
                Text::new(vetted_text)
            ];

            container = container.push(
                Container::new(column)
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

        // TODO: Add a function to `Wallet` to check whether we can safely leave a federation.
        // Call it here rather and get rid of `has_zero_balance`.
        let has_zero_balance = self.view.balance.msats == 0;

        if !has_zero_balance {
            container = container.push(
                Text::new("Must have a zero balance in this federation in order to leave.")
                    .size(20),
            );
        }

        container = container.push(
            icon_button("Leave Federation", SvgIcon::Delete, PaletteColor::Danger).on_press_maybe(
                has_zero_balance.then(|| {
                    app::Message::Routes(routes::Message::BitcoinWalletPage(
                        super::Message::FederationDetails(Message::LeaveFederation(
                            self.view.federation_id,
                        )),
                    ))
                }),
            ),
        );

        container = container.push(
            icon_button("Back", SvgIcon::ArrowBack, PaletteColor::Background).on_press(
                app::Message::Routes(routes::Message::Navigate(routes::RouteName::BitcoinWallet(
                    super::SubrouteName::List,
                ))),
            ),
        );

        container
    }
}
