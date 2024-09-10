use std::{collections::BTreeMap, sync::Arc};

use fedimint_core::{config::FederationId, Amount};
use fedimint_ln_common::bitcoin::Denomination;
use iced::{
    widget::{combo_box, qr_code::Data, text_input, Column, QRCode, Text},
    Task,
};
use lightning_invoice::Bolt11Invoice;

use crate::{
    app,
    fedimint::{FederationView, LightningReceiveCompletion, Wallet},
    routes::{container, Loadable, RouteName},
    ui_components::{icon_button, PaletteColor, SvgIcon},
    ConnectedState,
};

use super::SubrouteName;

#[derive(Debug, Clone)]
pub enum Message {
    // Invoice creation fields.
    AmountInputChanged(String),
    DenominationComboBoxSelected(Denomination),
    FederationComboBoxSelected(FederationView),

    // Invoice creation and payment.
    CreateInvoice(Amount, FederationId),
    InvoiceCreated(Bolt11Invoice),
    FailedToCreateInvoice,
    PaymentSuccess(Bolt11Invoice),
    PaymentFailure(Bolt11Invoice),

    UpdateFederationViews(BTreeMap<FederationId, FederationView>),
}

pub struct Page {
    wallet: Arc<Wallet>,
    amount_input: String,
    denomination_combo_box_state: combo_box::State<Denomination>,
    denomination_combo_box_selected_denomination: Option<Denomination>,
    federation_combo_box_state: combo_box::State<FederationView>,
    federation_combo_box_selected_federation: Option<FederationView>,
    loadable_lightning_invoice_data_or: Option<Loadable<(Bolt11Invoice, Data, Loadable<()>)>>,
}

impl Page {
    pub fn new(connected_state: &ConnectedState) -> Self {
        Self {
            wallet: connected_state.wallet.clone(),
            amount_input: String::new(),
            denomination_combo_box_state: combo_box::State::new(vec![
                Denomination::MilliSatoshi,
                Denomination::Satoshi,
                Denomination::Bitcoin,
            ]),
            denomination_combo_box_selected_denomination: Some(Denomination::Satoshi),
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
            loadable_lightning_invoice_data_or: None,
        }
    }

    pub fn update(&mut self, msg: Message) -> Task<app::Message> {
        match msg {
            Message::AmountInputChanged(new_amount_input) => {
                self.amount_input = new_amount_input;

                Task::none()
            }
            Message::DenominationComboBoxSelected(denomination) => {
                self.denomination_combo_box_selected_denomination = Some(denomination);

                Task::none()
            }
            Message::FederationComboBoxSelected(federation) => {
                self.federation_combo_box_selected_federation = Some(federation);

                Task::none()
            }
            Message::CreateInvoice(amount, federation_id) => {
                self.loadable_lightning_invoice_data_or = Some(Loadable::Loading);

                let wallet = self.wallet.clone();

                Task::stream(async_stream::stream! {
                    match wallet
                        .receive_payment(federation_id, amount, String::new())
                        .await
                    {
                        Ok((invoice, payment_completion_receiver)) => {
                            yield app::Message::BitcoinWalletPage(super::Message::Receive(
                                Message::InvoiceCreated(
                                invoice.clone(),
                            )));

                            match payment_completion_receiver.await {
                                Ok(lightning_receive_completion) => {
                                    match lightning_receive_completion {
                                        LightningReceiveCompletion::Success => {
                                            yield app::Message::BitcoinWalletPage(super::Message::Receive(
                                                Message::PaymentSuccess(invoice)));
                                        }
                                        LightningReceiveCompletion::Failure => {
                                            yield app::Message::BitcoinWalletPage(super::Message::Receive(
                                                Message::PaymentFailure(invoice)));
                                        }
                                    }
                                }
                                Err(_) => {
                                    println!("Payment receive completion receiver was cancelled. This is a bug!");
                                }
                            };
                        }
                        Err(_) => {
                            yield app::Message::BitcoinWalletPage(super::Message::Receive(
                                Message::FailedToCreateInvoice));
                        }
                    }
                })
            }
            Message::InvoiceCreated(invoice) => {
                let new_qr_code_data = Data::new(invoice.to_string()).unwrap();

                self.loadable_lightning_invoice_data_or = Some(Loadable::Loaded((
                    invoice,
                    new_qr_code_data,
                    Loadable::Loading,
                )));

                Task::none()
            }
            Message::FailedToCreateInvoice => {
                self.loadable_lightning_invoice_data_or = Some(Loadable::Failed);

                Task::none()
            }
            Message::PaymentSuccess(succeeded_invoice) => {
                if let Some(Loadable::Loaded((invoice, _, loadable_invoice_payment))) =
                    &mut self.loadable_lightning_invoice_data_or
                {
                    if invoice == &succeeded_invoice {
                        *loadable_invoice_payment = Loadable::Loaded(());
                    }
                }

                Task::none()
            }
            Message::PaymentFailure(failed_invoice) => {
                if let Some(Loadable::Loaded((invoice, _, loadable_invoice_payment))) =
                    &mut self.loadable_lightning_invoice_data_or
                {
                    if invoice == &failed_invoice {
                        *loadable_invoice_payment = Loadable::Failed;
                    }
                }

                Task::none()
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
        let mut container = container("Receive");

        let amount_or = self
            .denomination_combo_box_selected_denomination
            .and_then(|denomination| Amount::from_str_in(&self.amount_input, denomination).ok());

        // If the inputted amount to receive is valid and a federation
        // is selected, then we can proceed to pay the invoice.
        let parsed_amount_and_selected_federation_id_or = amount_or.and_then(|invoice| {
            self.federation_combo_box_selected_federation
                .as_ref()
                .map(|selected_federation| (invoice, selected_federation.federation_id))
        });

        container = container
            .push(
                text_input("Amount to receive", &self.amount_input)
                    .on_input(|input| {
                        app::Message::BitcoinWalletPage(super::Message::Receive(
                            Message::AmountInputChanged(input),
                        ))
                    })
                    .padding(10)
                    .size(30),
            )
            .push(combo_box(
                &self.denomination_combo_box_state,
                "Denomination",
                self.denomination_combo_box_selected_denomination.as_ref(),
                Self::on_denomination_combo_box_change,
            ))
            .push(combo_box(
                &self.federation_combo_box_state,
                "Federation to receive to",
                self.federation_combo_box_selected_federation.as_ref(),
                Self::on_federation_combo_box_change,
            ));

        container = if let Some(loadable_lightning_invoice_data) =
            &self.loadable_lightning_invoice_data_or
        {
            match loadable_lightning_invoice_data {
                Loadable::Loading => container.push(Text::new("Loading...")),
                Loadable::Loaded((lightning_invoice, qr_code_data, is_paid)) => {
                    if is_paid == &Loadable::Loaded(()) {
                        container.push(Text::new("Payment successful!"))
                    } else {
                        container.push(QRCode::new(qr_code_data)).push(
                            icon_button(
                                "Copy Invoice",
                                SvgIcon::ContentCopy,
                                PaletteColor::Primary,
                            )
                            .on_press(
                                app::Message::CopyStringToClipboard(lightning_invoice.to_string()),
                            ),
                        )
                    }
                }
                Loadable::Failed => container.push(Text::new("Failed to create invoice")),
            }
        } else {
            container.push(
                icon_button("Create Invoice", SvgIcon::Send, PaletteColor::Primary).on_press_maybe(
                    parsed_amount_and_selected_federation_id_or.map(|(amount, federation_id)| {
                        app::Message::BitcoinWalletPage(super::Message::Receive(
                            Message::CreateInvoice(amount, federation_id),
                        ))
                    }),
                ),
            )
        };

        container = container.push(
            icon_button("Back", SvgIcon::ArrowBack, PaletteColor::Background).on_press(
                app::Message::Navigate(RouteName::BitcoinWallet(SubrouteName::List)),
            ),
        );

        container
    }

    fn on_denomination_combo_box_change(denomination: Denomination) -> app::Message {
        app::Message::BitcoinWalletPage(super::Message::Receive(
            Message::DenominationComboBoxSelected(denomination),
        ))
    }

    fn on_federation_combo_box_change(federation_view: FederationView) -> app::Message {
        app::Message::BitcoinWalletPage(super::Message::Receive(
            Message::FederationComboBoxSelected(federation_view),
        ))
    }
}
