// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use async_trait::async_trait;
use nip_70::{Nip70, Nip70Server, Nip70ServerError, RelayPolicy};
use nostr_sdk::event::{Event, UnsignedEvent};
use nostr_sdk::Keys;
use secp256k1::XOnlyPublicKey;
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{CustomMenuItem, Manager, SystemTray, SystemTrayEvent, SystemTrayMenu, WindowEvent};
use tokio::sync::Mutex;

const TRAY_MENU_ITEM_QUIT: &str = "quit";

struct KeystacheNip70 {
    /// The key pair used to sign events.
    keys: Keys,

    /// Map of hex-encoded event IDs to channels for signaling when the signing of an event has been approved/rejected.
    in_progress_event_signings: Mutex<HashMap<String, tokio::sync::oneshot::Sender<bool>>>,

    /// Handle to the Tauri application. Used to emit events.
    app_handle: tauri::AppHandle,
}

impl KeystacheNip70 {
    // TODO: Remove this method and implement a way to load & store keys on disk.
    fn new_with_generated_keys(app_handle: tauri::AppHandle) -> Self {
        Self {
            keys: Keys::generate(),
            in_progress_event_signings: Mutex::new(HashMap::new()),
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
        let (tx, rx) = tokio::sync::oneshot::channel::<bool>();
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
            get_public_key
        ])
        .on_window_event(|global_window_event| {
            match global_window_event.event() {
                WindowEvent::CloseRequested { api, .. } => {
                    // Hide the window instead of closing it.
                    api.prevent_close();
                    let _ = global_window_event.window().hide();
                }
                _ => {}
            }
        })
        .setup(|app| {
            // Prevent the app from showing in the Dock on MacOS.
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let keystache_nip_70 = Arc::new(KeystacheNip70::new_with_generated_keys(app.handle()));
            let nip_70_server_or = Nip70Server::new(keystache_nip_70.clone()).ok();
            app.manage(keystache_nip_70);
            app.manage(nip_70_server_or);

            let handle = app.handle();
            let tray_menu = SystemTrayMenu::new()
                .add_item(CustomMenuItem::new(TRAY_MENU_ITEM_QUIT, "Quit Keystache"));
            SystemTray::new()
                .with_menu(tray_menu)
                .on_event(move |event| match event {
                    SystemTrayEvent::LeftClick { .. } => {
                        // Toggle visibility of the main window.
                        // Note that `set_focus()` makes the window visible if it's hidden.
                        if let Some(window) = handle.get_window("main") {
                            if let Ok(is_visible) = window.is_visible() {
                                if is_visible {
                                    let _ = window.hide();
                                } else {
                                    let _ = window.set_focus();
                                }
                            }
                        }
                    }
                    SystemTrayEvent::MenuItemClick { id, .. } => {
                        if id == TRAY_MENU_ITEM_QUIT {
                            handle.exit(0);
                        }
                    }
                    _ => {}
                })
                .build(app)?;

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
