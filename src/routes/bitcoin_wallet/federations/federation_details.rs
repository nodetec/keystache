use iced::{
    widget::{column, container::Style, Column, Container, Text},
    Border, Length, Shadow, Theme,
};

use crate::{
    app,
    fedimint::FederationView,
    routes::{self, bitcoin_wallet, container},
    ui_components::{icon_button, PaletteColor, SvgIcon},
    util::{format_amount, lighten, truncate_text},
};

pub struct Page {
    federation_view: FederationView,
}

impl Page {
    pub fn new(view: FederationView) -> Self {
        Self {
            federation_view: view,
        }
    }

    pub fn federation_view(&self) -> &FederationView {
        &self.federation_view
    }

    pub fn view<'a>(&self) -> Column<'a, app::Message> {
        let mut container = container("Federation Details")
            .push(
                Text::new(
                    self.federation_view
                        .name_or
                        .clone()
                        .unwrap_or_else(|| "Unnamed Federation".to_string()),
                )
                .size(25),
            )
            .push(Text::new(format!(
                "Federation ID: {}",
                truncate_text(&self.federation_view.federation_id.to_string(), 23, true)
            )))
            .push(Text::new(format_amount(self.federation_view.balance)))
            .push(Text::new("Gateways").size(20));

        for gateway in &self.federation_view.gateways {
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
        let has_zero_balance = self.federation_view.balance.msats == 0;

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
                        bitcoin_wallet::Message::LeaveFederation(
                            self.federation_view.federation_id,
                        ),
                    ))
                }),
            ),
        );

        container = container.push(
            icon_button("Back", SvgIcon::ArrowBack, PaletteColor::Background).on_press(
                app::Message::Routes(routes::Message::Navigate(routes::RouteName::BitcoinWallet(
                    bitcoin_wallet::SubrouteName::Main,
                ))),
            ),
        );

        container
    }
}
