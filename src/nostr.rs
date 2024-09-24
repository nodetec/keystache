use std::collections::BTreeMap;
use std::fmt::Debug;
use std::time::Duration;

use iced::Subscription;
use nostr_relay_pool::RelayStatus;
use nostr_sdk::Url;

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct NostrState {
    pub relay_connections: BTreeMap<Url, RelayStatus>,
}

#[derive(Debug, Clone)]
pub enum NostrModuleMessage {
    ConnectToRelay(String),
    DisconnectFromRelay(String),
}

#[derive(Clone)]
pub struct NostrModule {
    client: nostr_sdk::Client,
}

impl NostrModule {
    pub fn new() -> Self {
        Self {
            client: nostr_sdk::Client::default(),
        }
    }

    pub fn update(&self, message: NostrModuleMessage) {
        match message {
            NostrModuleMessage::ConnectToRelay(url) => {
                let client = self.client.clone();

                tokio::spawn(async move {
                    client.add_relay(&url).await.unwrap();
                    client.connect_relay(url).await.unwrap();
                });
            }
            NostrModuleMessage::DisconnectFromRelay(url) => {
                let client = self.client.clone();

                tokio::spawn(async move {
                    client.remove_relay(&url).await.unwrap();
                });
            }
        }
    }

    pub fn subscription(&self) -> Subscription<NostrState> {
        const POLL_DURATION: Duration = Duration::from_millis(200);

        let client = self.client.clone();

        Subscription::run_with_id(
            std::any::TypeId::of::<NostrState>(),
            // We're wrapping `stream` in a `stream!` macro to make it lazy (meaning `stream` isn't
            // created unless the outer `stream!` is actually used). This is necessary because the
            // outer `stream!` is created on every update, but will only be polled if the subscription
            // ID is new.
            async_stream::stream! {
                let mut last_state = NostrState::default();
                loop {
                    let new_state = Self::get_state(&client).await;
                    if new_state != last_state {
                        yield new_state.clone();
                        last_state = new_state;
                    }

                    tokio::time::sleep(POLL_DURATION).await;
                }
            },
        )
    }

    /// Fetches the current state of the Nostr SDK client.
    /// Note: This is async because it's grabbing read locks
    /// on the relay `RwLock`s. No network requests are made.
    async fn get_state(client: &nostr_sdk::Client) -> NostrState {
        let mut relay_connections = BTreeMap::new();

        for (url, relay) in client.relays().await {
            relay_connections.insert(url.clone(), relay.status().await);
        }

        NostrState { relay_connections }
    }
}
