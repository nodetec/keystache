use iced::{
    widget::{container::Style, horizontal_space, row, Column, Container, Text},
    Border, Length, Shadow, Theme,
};

use crate::{
    app,
    routes::{self, bitcoin_wallet, container, ConnectedState, Loadable},
    ui_components::{icon_button, PaletteColor, SvgIcon},
    util::{format_amount, lighten},
};

pub struct Page {
    connected_state: ConnectedState,
}

impl Page {
    pub fn new(connected_state: &ConnectedState) -> Self {
        Self {
            connected_state: connected_state.clone(),
        }
    }

    pub fn view<'a>(&self) -> Column<'a, app::Message> {
        let mut container = container("Federations");

        match &self.connected_state.loadable_wallet_view {
            Loadable::Loading => {
                container = container.push(Text::new("Loading federations...").size(25));
            }
            Loadable::Loaded(wallet_view) => {
                for federation_view in wallet_view.federations.values() {
                    let column: Column<_, Theme, _> = Column::new()
                        .push(
                            Text::new(
                                federation_view
                                    .name_or
                                    .clone()
                                    .unwrap_or_else(|| "Unnamed Federation".to_string()),
                            )
                            .size(25),
                        )
                        .push(Text::new(format_amount(federation_view.balance)));

                    container = container.push(
                        Container::new(row![
                            column,
                            horizontal_space(),
                            icon_button("Details", SvgIcon::ChevronRight, PaletteColor::Background)
                                .on_press(app::Message::Routes(routes::Message::Navigate(
                                    routes::RouteName::BitcoinWallet(
                                        bitcoin_wallet::SubrouteName::Federations(
                                            super::SubrouteName::FederationDetails(
                                                federation_view.clone()
                                            )
                                        )
                                    )
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

        container
    }
}
