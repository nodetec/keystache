// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod database;

use async_trait::async_trait;
use database::Database;
use nip_70::{
    run_nip70_server, Nip70, Nip70ServerError, PayInvoiceRequest, PayInvoiceResponse, RelayPolicy,
};
use nostr_sdk::event::{Event, UnsignedEvent};
use nostr_sdk::key::{KeyPair, Secp256k1, SecretKey, XOnlyPublicKey};
use nostr_sdk::Keys;
use nostr_sdk::FromBech32;
use std::collections::HashMap;
use std::sync::Arc;
use tauri::Manager;
use tokio::sync::Mutex;

struct KeystacheNip70 {
    /// Database handle. `None` if there was an error opening the database, otherwise `Some`.
    database_or: Option<Database>,

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
    fn new(app_handle: tauri::AppHandle) -> Self {
        Self {
            database_or: Database::new_in_app_data_dir(app_handle.clone(), None).ok(),
            in_progress_event_signings: Mutex::new(HashMap::new()),
            in_progress_invoice_payments: Mutex::new(HashMap::new()),
            app_handle,
        }
    }

    /// Wipe all existing keypairs and save a new one.
    /// TODO: Once we support multiple keypairs, we should remove this.
    fn set_keypair(&self, keypair: KeyPair) -> Result<(), Nip70ServerError> {
        let database = match &self.database_or {
            Some(database) => database,
            None => return Err(Nip70ServerError::InternalError),
        };

        // Wipe all existing keypairs.
        // TODO: Hardcoding the limit here isn't very robust. Should we allow for
        // setting it to `None` to allow for iterating through all keypairs?
        for keypair in database.list_keypairs(10_000, 0).map_err(|_| Nip70ServerError::InternalError)? {
            database.remove_keypair(&keypair.x_only_public_key().0).map_err(|_| {
                Nip70ServerError::InternalError
            })?;
        }

        // Save the new keypair.
        database
            .save_keypair(&keypair)
            .map_err(|_| Nip70ServerError::InternalError)
    }
}

#[async_trait]
impl Nip70 for KeystacheNip70 {
    async fn get_public_key(&self) -> Result<XOnlyPublicKey, Nip70ServerError> {
        let database = match &self.database_or {
            Some(database) => database,
            None => return Err(Nip70ServerError::InternalError),
        };

        match database.get_first_public_key() {
            Ok(Some(public_key)) => Ok(public_key),
            _ => Err(Nip70ServerError::InternalError),
        }
    }

    async fn sign_event(&self, event: UnsignedEvent) -> Result<Event, Nip70ServerError> {
        let database = match &self.database_or {
            Some(database) => database,
            None => return Err(Nip70ServerError::InternalError),
        };

        let keypair = match database.get_first_keypair() {
            Ok(Some(keypair)) => keypair,
            _ => return Err(Nip70ServerError::InternalError),
        };

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
                .sign(&Keys::new(keypair.secret_key()))
                .map_err(|_| Nip70ServerError::InternalError)
        } else {
            Err(Nip70ServerError::Rejected)
        }
    }

    async fn pay_invoice(
        &self,
        pay_invoice_request: PayInvoiceRequest,
    ) -> Result<PayInvoiceResponse, Nip70ServerError> {
        let invoice = pay_invoice_request.invoice().to_string();

        let (tx, rx) = tokio::sync::oneshot::channel();
        self.in_progress_invoice_payments
            .lock()
            .await
            .insert(invoice.clone(), tx);

        self.app_handle
            .emit_all("pay_invoice_request", invoice)
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

#[tauri::command]
async fn set_nsec(
    nsec: String,
    state: tauri::State<'_, Arc<KeystacheNip70>>,
) -> anyhow::Result<(), String> {
    let keypair = SecretKey::from_bech32(nsec).map_err(|_| "Error parsing nsec")?.keypair(&Secp256k1::new());
    state.set_keypair(keypair).map_err(|_| "Error setting keypair")?;
    Ok(())
}

#[tokio::main]
async fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            respond_to_sign_event_request,
            respond_to_pay_invoice_request,
            get_public_key,
            set_nsec
        ])
        .setup(|app| {
            let keystache_nip_70 = Arc::new(KeystacheNip70::new(app.handle()));
            let nip_70_server_or = run_nip70_server(keystache_nip_70.clone()).ok();
            app.manage(keystache_nip_70);
            app.manage(nip_70_server_or);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
