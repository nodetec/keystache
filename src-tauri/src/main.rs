// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use async_trait::async_trait;
use lightning_invoice::Bolt11Invoice;
use nip_70::{
    Nip70, Nip70Server, Nip70ServerError, PayInvoiceRequest, PayInvoiceResponse, RelayPolicy,
};
use nostr_sdk::event::{Event, UnsignedEvent};
use nostr_sdk::Keys;
use secp256k1::XOnlyPublicKey;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use tauri::Manager;
use tokio::sync::Mutex;

struct KeystacheNip70 {
    /// The key pair used to sign events.
    keys: Keys,

    /// Map of hex-encoded event IDs to channels for signaling when the signing of an event has been approved/rejected.
    in_progress_event_signings: Mutex<HashMap<String, tokio::sync::oneshot::Sender<bool>>>,

    /// Map of Bolt11 invoice strings to channels for signaling when the payment of an invoice has been paid/failed/rejected.
    in_progress_invoice_payments: Mutex<
        HashMap<String, tokio::sync::oneshot::Sender<Result<PayInvoiceResponse, Nip70ServerError>>>,
    >,

    /// Handle to the Tauri application. Used to emit events.
    app_handle: tauri::AppHandle,
}

impl KeystacheNip70 {
    // TODO: Remove this method and implement a way to load & store keys on disk.
    fn new_with_generated_keys(app_handle: tauri::AppHandle) -> Self {
        Self {
            keys: Keys::generate(),
            in_progress_event_signings: Mutex::new(HashMap::new()),
            in_progress_invoice_payments: Mutex::new(HashMap::new()),
            app_handle,
        }
    }
}

#[async_trait]
impl Nip70 for KeystacheNip70 {
    async fn get_public_key(&self) -> Result<XOnlyPublicKey, Nip70ServerError> {
        Ok(self.keys.public_key())
    }

    async fn sign_event(&self, event: UnsignedEvent) -> Result<Event, Nip70ServerError> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.in_progress_event_signings
            .lock()
            .await
            .insert(event.id.to_hex(), tx);

        self.app_handle
            .emit_all("sign_event_request", event.clone())
            .map_err(|_err| Nip70ServerError::InternalError)?;

        let signing_approved = rx.await.unwrap_or(false);

        if signing_approved {
            event
                .sign(&self.keys)
                .map_err(|_| Nip70ServerError::InternalError)
        } else {
            Err(Nip70ServerError::Rejected)
        }
    }

    async fn pay_invoice(
        &self,
        pay_invoice_request: PayInvoiceRequest,
    ) -> Result<PayInvoiceResponse, Nip70ServerError> {
        // Return early if the invoice is malformed. We don't actually
        // need the parsed invoice, we just want to check if it's valid.
        match Bolt11Invoice::from_str(pay_invoice_request.invoice()) {
            Err(_) => return Ok(PayInvoiceResponse::ErrorMalformedInvoice),
            _ => {}
        };

        let (tx, rx) = tokio::sync::oneshot::channel();
        self.in_progress_invoice_payments
            .lock()
            .await
            .insert(pay_invoice_request.invoice().to_string(), tx);

        self.app_handle
            .emit_all("pay_invoice_request", pay_invoice_request.invoice())
            .map_err(|_err| Nip70ServerError::InternalError)?;

        rx.await
            .unwrap_or_else(|_| Err(Nip70ServerError::InternalError))
    }

    async fn get_relays(
        &self,
    ) -> Result<Option<std::collections::HashMap<String, RelayPolicy>>, Nip70ServerError> {
        // TODO: Implement relay support.
        Ok(None)
    }
}

#[tauri::command]
async fn respond_to_sign_event_request(
    event_id: String,
    approved: bool,
    state: tauri::State<'_, Arc<KeystacheNip70>>,
) -> Result<(), ()> {
    if let Some(tx) = state
        .in_progress_event_signings
        .lock()
        .await
        .remove(&event_id)
    {
        let _ = tx.send(approved);
    }

    Ok(())
}

#[tauri::command]
async fn respond_to_pay_invoice_request(
    invoice: String,
    outcome: &str,
    state: tauri::State<'_, Arc<KeystacheNip70>>,
) -> Result<(), ()> {
    if let Some(tx) = state
        .in_progress_invoice_payments
        .lock()
        .await
        .remove(&invoice)
    {
        let response = match outcome {
            "paid" => Ok(PayInvoiceResponse::Success(
                "TODO: Insert preimage here".to_string(),
            )),
            "failed" => {
                Ok(PayInvoiceResponse::ErrorPaymentFailed(
                    // TODO: This should be a more descriptive error.
                    "Unknown client-side error".to_string(),
                ))
            }
            "rejected" => Err(Nip70ServerError::Rejected),
            _ => Err(Nip70ServerError::InternalError),
        };
        let _ = tx.send(response);
    }

    Ok(())
}

#[tauri::command]
async fn get_public_key(
    state: tauri::State<'_, Arc<KeystacheNip70>>,
) -> Result<XOnlyPublicKey, String> {
    state
        .get_public_key()
        .await
        .map_err(|err| format!("Error: {:?}", err))
}

#[tokio::main]
async fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            respond_to_sign_event_request,
            respond_to_pay_invoice_request,
            get_public_key
        ])
        .setup(|app| {
            let keystache_nip_70 = Arc::new(KeystacheNip70::new_with_generated_keys(app.handle()));
            let nip_70_server_or = Nip70Server::new(keystache_nip_70.clone()).ok();
            app.manage(keystache_nip_70);
            app.manage(nip_70_server_or);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
