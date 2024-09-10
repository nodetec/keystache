use std::{collections::BTreeMap, str::FromStr, sync::Arc};

use fedimint_core::config::FederationId;
use iced::{
    widget::{combo_box, text_input, Column, Text},
    Task,
};
use lightning_invoice::Bolt11Invoice;

use crate::{
    app,
    fedimint::{FederationView, Wallet},
    routes::{self, container, Loadable, RouteName},
    ui_components::{icon_button, PaletteColor, SvgIcon, Toast, ToastStatus},
};

use super::{ConnectedState, SubrouteName};

#[derive(Debug, Clone)]
pub enum Message {
    // Payment input fields.
    LightningInvoiceInputChanged(String),
    FederationComboBoxSelected(FederationView),

    // Payment actions.
    PayInvoice(Bolt11Invoice, FederationId),
    PayInvoiceSucceeded(Bolt11Invoice),
    PayInvoiceFailed((Bolt11Invoice, Arc<anyhow::Error>)),

    UpdateFederationViews(BTreeMap<FederationId, FederationView>),
}

pub struct Page {
    wallet: Arc<Wallet>,
    lightning_invoice_input: String,
    federation_combo_box_state: combo_box::State<FederationView>,
    federation_combo_box_selected_federation: Option<FederationView>,
    loadable_invoice_payment_or: Option<Loadable<()>>,
}

impl Page {
    pub fn new(connected_state: &ConnectedState) -> Self {
        Self {
            wallet: connected_state.wallet.clone(),
            lightning_invoice_input: String::new(),
            federation_combo_box_state: combo_box::State::new(
                connected_state
                    .loadable_federation_views
                    .as_ref_option()
                    .cloned()
                    .unwrap_or_default()
                    .into_values()
                    .collect(),
            ),
            federation_combo_box_selected_federation: None,
            loadable_invoice_payment_or: None,
        }
    }

    pub fn update(&mut self, msg: Message) -> Task<app::Message> {
        match msg {
            Message::LightningInvoiceInputChanged(new_lightning_invoice_input) => {
                self.lightning_invoice_input = new_lightning_invoice_input;

                Task::none()
            }
            Message::FederationComboBoxSelected(federation) => {
                self.federation_combo_box_selected_federation = Some(federation);

                Task::none()
            }
            Message::PayInvoice(invoice, federation_id) => {
                self.loadable_invoice_payment_or = Some(Loadable::Loading);

                let wallet = self.wallet.clone();

                Task::future(async move {
                    match wallet.pay_invoice(invoice.clone(), federation_id).await {
                        Ok(()) => app::Message::Routes(routes::Message::BitcoinWalletPage(
                            super::Message::Send(Message::PayInvoiceSucceeded(invoice)),
                        )),
                        Err(err) => app::Message::Routes(routes::Message::BitcoinWalletPage(
                            super::Message::Send(Message::PayInvoiceFailed((
                                invoice,
                                Arc::from(err),
                            ))),
                        )),
                    }
                })
            }
            Message::PayInvoiceSucceeded(invoice) => {
                let invoice_or = Bolt11Invoice::from_str(&self.lightning_invoice_input).ok();

                if Some(invoice) == invoice_or {
                    self.loadable_invoice_payment_or = Some(Loadable::Loaded(()));
                }

                Task::done(app::Message::AddToast(Toast {
                    title: "Payment succeeded".to_string(),
                    body: "Invoice was successfully paid".to_string(),
                    status: ToastStatus::Good,
                }))
            }
            Message::PayInvoiceFailed((invoice, err)) => {
                let invoice_or = Bolt11Invoice::from_str(&self.lightning_invoice_input).ok();

                if Some(invoice) == invoice_or {
                    self.loadable_invoice_payment_or = Some(Loadable::Failed);
                }

                Task::done(app::Message::AddToast(Toast {
                    title: "Payment failed".to_string(),
                    body: format!("Failed to pay invoice: {err}"),
                    status: ToastStatus::Bad,
                }))
            }
            Message::UpdateFederationViews(federation_views) => {
                self.federation_combo_box_selected_federation = self
                    .federation_combo_box_selected_federation
                    .as_ref()
                    .and_then(|selected_federation| {
                        federation_views
                            .get(&selected_federation.federation_id)
                            .cloned()
                    });

                self.federation_combo_box_state =
                    combo_box::State::new(federation_views.into_values().collect());

                Task::none()
            }
        }
    }

    pub fn view(&self) -> Column<app::Message> {
        let mut container = container("Send");

        let invoice_or = Bolt11Invoice::from_str(&self.lightning_invoice_input).ok();

        // If the inputted invoice is valid and a federation is
        // selected, then we can proceed to pay the invoice.
        let parsed_invoice_and_selected_federation_id_or = invoice_or.and_then(|invoice| {
            self.federation_combo_box_selected_federation
                .as_ref()
                .map(|selected_federation| (invoice, selected_federation.federation_id))
        });

        container = match &self.loadable_invoice_payment_or {
            Some(Loadable::Loading) => container.push(Text::new("Loading...")),
            Some(Loadable::Loaded(())) => container.push(Text::new("Payment successful!")),
            Some(Loadable::Failed) => container.push(Text::new("Payment failed")),
            None => container
                .push(
                    text_input("Lightning Invoice", &self.lightning_invoice_input)
                        .on_input(|input| {
                            app::Message::Routes(routes::Message::BitcoinWalletPage(
                                super::Message::Send(Message::LightningInvoiceInputChanged(input)),
                            ))
                        })
                        .padding(10)
                        .size(30),
                )
                .push(combo_box(
                    &self.federation_combo_box_state,
                    "Federation to pay from",
                    self.federation_combo_box_selected_federation.as_ref(),
                    Self::on_combo_box_change,
                ))
                .push(
                    icon_button("Pay Invoice", SvgIcon::Send, PaletteColor::Primary)
                        .on_press_maybe(parsed_invoice_and_selected_federation_id_or.map(
                            |(invoice, federation_id)| {
                                app::Message::Routes(routes::Message::BitcoinWalletPage(
                                    super::Message::Send(Message::PayInvoice(
                                        invoice,
                                        federation_id,
                                    )),
                                ))
                            },
                        )),
                ),
        };

        container = container.push(
            icon_button("Back", SvgIcon::ArrowBack, PaletteColor::Background).on_press(
                app::Message::Routes(routes::Message::Navigate(RouteName::BitcoinWallet(
                    SubrouteName::List,
                ))),
            ),
        );

        container
    }

    fn on_combo_box_change(federation_view: FederationView) -> app::Message {
        app::Message::Routes(routes::Message::BitcoinWalletPage(super::Message::Send(
            Message::FederationComboBoxSelected(federation_view),
        )))
    }
}
