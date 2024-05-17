// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod database;

use async_trait::async_trait;
use database::Database;
use lightning_invoice::Bolt11Invoice;
use nip_55::nip46::{Nip46OverNip55Server, Nip46RequestApproval, Nip46RequestApprover};
use nip_55::KeyManager;
use nostr_sdk::key::SecretKey;
use nostr_sdk::nips::nip46;
use nostr_sdk::secp256k1::{Keypair, Secp256k1};
use nostr_sdk::{EventId, FromBech32, PublicKey, ToBech32};
use std::collections::HashMap;
use std::sync::Arc;
use tauri::Manager;
use tokio::sync::Mutex;

struct KeystacheKeyManager {
    /// Database handle. `None` if there was an error opening the database, otherwise `Some`.
    database_or: Option<Database>,
}

impl KeystacheKeyManager {
    fn new(app_handle: tauri::AppHandle) -> Self {
        Self {
            database_or: Database::new_in_app_data_dir(app_handle.clone(), None).ok(),
        }
    }

    /// Wipe all existing keypairs and save a new one.
    /// TODO: Once we support multiple keypairs, we should remove this.
    fn set_keypair(&self, keypair: Keypair) -> anyhow::Result<()> {
        let database = match &self.database_or {
            Some(database) => database,
            None => return Err(anyhow::Error::msg("No database available")),
        };

        // Wipe all existing keypairs.
        // TODO: Hardcoding the limit here isn't very robust. Should we allow for
        // setting it to `None` to allow for iterating through all keypairs?
        for keypair in database.list_keypairs(10_000, 0)? {
            database.remove_keypair(&keypair.x_only_public_key().0.into())?;
        }

        // Save the new keypair.
        database.save_keypair(&keypair)
    }

    fn get_public_key(&self) -> anyhow::Result<Option<PublicKey>> {
        let database = match &self.database_or {
            Some(database) => database,
            None => return Err(anyhow::Error::msg("No database available")),
        };
        database.get_first_public_key()
    }
}

#[async_trait]
impl KeyManager for KeystacheKeyManager {
    fn get_secret_key(&self, public_key: &PublicKey) -> Option<SecretKey> {
        let database = match &self.database_or {
            Some(database) => database,
            None => return None,
        };
        // TODO: Fetch the secret key using the public key rather than iterating through all keypairs.
        let keypairs = database.list_keypairs(999, 0).ok()?;
        keypairs
            .into_iter()
            .find(|keypair| keypair.x_only_public_key().0 == **public_key)
            .map(|keypair| keypair.secret_key().into())
    }
}

struct KeystacheRequestApprover {
    /// Map of hex-encoded event IDs to channels for signaling when the signing of an event has been approved/rejected.
    in_progress_event_signings:
        Mutex<HashMap<String, tokio::sync::oneshot::Sender<Nip46RequestApproval>>>,

    /// Map of Bolt11 invoice strings to channels for signaling when the payment of an invoice has been paid/failed/rejected.
    in_progress_invoice_payments:
        Mutex<HashMap<String, tokio::sync::oneshot::Sender<Nip46RequestApproval>>>,

    /// Handle to the Tauri application. Used to emit events.
    app_handle: tauri::AppHandle,
}

impl KeystacheRequestApprover {
    fn new(app_handle: tauri::AppHandle) -> Self {
        Self {
            in_progress_event_signings: Mutex::new(HashMap::new()),
            in_progress_invoice_payments: Mutex::new(HashMap::new()),
            app_handle,
        }
    }

    async fn pay_invoice(&self, invoice: Bolt11Invoice) -> anyhow::Result<Nip46RequestApproval> {
        let invoice_string = invoice.to_string();

        let (tx, rx) = tokio::sync::oneshot::channel();
        self.in_progress_invoice_payments
            .lock()
            .await
            .insert(invoice_string.clone(), tx);

        self.app_handle
            .emit_all("pay_invoice_request", invoice_string)?;

        Ok(rx.await?)
    }
}

#[async_trait]
impl Nip46RequestApprover for KeystacheRequestApprover {
    async fn handle_batch_request(
        &self,
        requests: Vec<(nip46::Request, PublicKey)>,
    ) -> Nip46RequestApproval {
        // TODO: IMPORTANT!!! Currently we ignore all but the first request. We should handle all requests.
        // TODO: We should use `_user_pubkey` and pass it to the frontend.
        let (request, user_pubkey) = match requests.into_iter().next() {
            Some(request) => request,
            None => return Nip46RequestApproval::Reject,
        };

        // TODO: Handle more than just signing events.
        let mut event = match request {
            nip46::Request::SignEvent(event) => event,
            _ => return Nip46RequestApproval::Reject,
        };

        // TODO: Is this seriously the best way to do this?!
        let event_id = EventId::new(
            &event.pubkey,
            event.created_at,
            &event.kind,
            &event.tags,
            &event.content,
        );

        event.id = Some(event_id);

        let (tx, rx) = tokio::sync::oneshot::channel();
        self.in_progress_event_signings
            .lock()
            .await
            .insert(event_id.to_hex(), tx);

        if self
            .app_handle
            .emit_all(
                "sign_event_request",
                (event, user_pubkey.to_bech32().unwrap()),
            )
            .is_err()
        {
            return Nip46RequestApproval::Reject;
        }

        rx.await.unwrap_or(Nip46RequestApproval::Reject)
    }
}

#[tauri::command]
async fn respond_to_sign_event_request(
    event_id: String,
    approved: bool,
    state: tauri::State<'_, Arc<KeystacheRequestApprover>>,
) -> Result<(), ()> {
    if let Some(tx) = state
        .in_progress_event_signings
        .lock()
        .await
        .remove(&event_id)
    {
        let approval = if approved {
            Nip46RequestApproval::Approve
        } else {
            Nip46RequestApproval::Reject
        };
        let _ = tx.send(approval);
    }

    Ok(())
}

#[tauri::command]
async fn respond_to_pay_invoice_request(
    invoice: String,
    approved: bool,
    state: tauri::State<'_, Arc<KeystacheRequestApprover>>,
) -> Result<(), ()> {
    if let Some(tx) = state
        .in_progress_invoice_payments
        .lock()
        .await
        .remove(&invoice)
    {
        let approval = if approved {
            Nip46RequestApproval::Approve
        } else {
            Nip46RequestApproval::Reject
        };
        let _ = tx.send(approval);
    }

    Ok(())
}

#[tauri::command]
async fn get_public_key(
    state: tauri::State<'_, Arc<KeystacheKeyManager>>,
) -> Result<PublicKey, String> {
    match state
        .get_public_key()
        .map_err(|err| format!("Error: {:?}", err))?
    {
        Some(public_key) => Ok(public_key),
        None => Err("No public key available".to_string()),
    }
}

#[tauri::command]
async fn set_nsec(
    nsec: String,
    state: tauri::State<'_, Arc<KeystacheKeyManager>>,
) -> Result<(), String> {
    let keypair = SecretKey::from_bech32(nsec)
        .map_err(|_| "Error parsing nsec")?
        .keypair(&Secp256k1::new());
    state
        .set_keypair(keypair)
        .map_err(|_| "Error setting keypair")?;
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
            let keystache_key_manager = Arc::new(KeystacheKeyManager::new(app.handle()));
            let keystache_request_approver = Arc::new(KeystacheRequestApprover::new(app.handle()));
            let nip_70_server_or = Nip46OverNip55Server::start(
                "/tmp/nip55-kind24133",
                keystache_key_manager.clone(),
                keystache_request_approver.clone(),
            )
            .ok();
            app.manage(keystache_key_manager);
            app.manage(keystache_request_approver);
            app.manage(nip_70_server_or);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
