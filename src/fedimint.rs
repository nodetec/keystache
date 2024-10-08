use std::{
    collections::{BTreeMap, HashMap},
    fmt::Display,
    path::PathBuf,
    sync::Arc,
    time::Duration,
};

use directories::ProjectDirs;
use fedimint_bip39::Bip39RootSecretStrategy;
use fedimint_client::{
    derivable_secret::DerivableSecret, secret::RootSecretStrategy, Client, ClientHandle,
};
use fedimint_core::{config::FederationId, db::Database, invite_code::InviteCode, Amount};
use fedimint_ln_client::{LightningClientModule, LnReceiveState};
use fedimint_ln_common::{LightningGateway, LightningGatewayAnnouncement};
use fedimint_rocksdb::RocksDb;
use lightning_invoice::{Bolt11Invoice, Bolt11InvoiceDescription, Description};
use nostr_sdk::{
    bip39::Mnemonic,
    bitcoin::{
        bip32::{ChildNumber, DerivationPath, Xpriv},
        secp256k1::Secp256k1,
        Network,
    },
};
use secp256k1::rand::{seq::SliceRandom, thread_rng};
use tokio::sync::{mpsc, oneshot, watch, Mutex, MutexGuard};
use tokio_stream::StreamExt;

use crate::util::format_amount;

const FEDIMINT_CLIENTS_DATA_DIR_NAME: &str = "fedimint_clients";

// TODO: Figure out if we even want this. If we do, it probably shouldn't live here.
// It'd make more sense for it to live wherever the key is maintained elsewhere, and
// have `Wallet::new()` assume that the key is already derived.
const FEDIMINT_DERIVATION_NUMBER: u32 = 1;

const WALLET_VIEW_UPDATE_INTERVAL: Duration = Duration::from_secs(5);

pub enum LightningReceiveCompletion {
    Success,
    Failure,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WalletView {
    pub federations: BTreeMap<FederationId, FederationView>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FederationView {
    pub federation_id: FederationId,
    pub name_or: Option<String>,
    pub balance: Amount,
    pub gateways: Vec<LightningGatewayAnnouncement>,
}

impl Display for FederationView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name_or_id = self
            .name_or
            .clone()
            .unwrap_or_else(|| self.federation_id.to_string());

        let balance = format_amount(self.balance);

        write!(f, "{name_or_id} ({balance})")
    }
}

pub struct Wallet {
    derivable_secret: DerivableSecret,
    clients: Arc<Mutex<HashMap<FederationId, ClientHandle>>>,
    fedimint_clients_data_dir: PathBuf,
    view_update_receiver: watch::Receiver<WalletView>,
    // Used to tell `Self.view_update_task` to immediately update the view.
    // If the view has changed, the task will yield a new view message.
    // Then the oneshot sender is used to tell the caller that the view
    // is now up to date (even if no new value was yielded).
    force_update_view_sender: mpsc::Sender<oneshot::Sender<()>>,
    view_update_task: tokio::task::JoinHandle<()>,
}

impl Drop for Wallet {
    fn drop(&mut self) {
        // TODO: We should properly shut down the task rather than aborting it.
        self.view_update_task.abort();
    }
}

impl Wallet {
    pub fn new(xprivkey: Xpriv, network: Network, project_dirs: &ProjectDirs) -> Self {
        let (view_update_sender, view_update_receiver) = watch::channel(WalletView {
            federations: BTreeMap::new(),
        });

        let (force_update_view_sender, mut force_update_view_receiver) =
            mpsc::channel::<oneshot::Sender<()>>(100);

        let clients = Arc::new(Mutex::new(HashMap::new()));

        let clients_clone = clients.clone();
        let view_update_task = tokio::spawn(async move {
            let mut last_state_or = None;

            // TODO: Optimize this. Repeated polling is not ideal.
            loop {
                // Wait either for a force update or for a timeout. If a force update
                // occurs, then `force_update_completed_oneshot_or` will be `Some`.
                // If a timeout occurs, then `force_update_completed_oneshot_or` will be `None`.
                let force_update_completed_oneshot_or = tokio::select! {
                    Some(force_update_completed_oneshot) = force_update_view_receiver.recv() => Some(force_update_completed_oneshot),
                    () = tokio::time::sleep(WALLET_VIEW_UPDATE_INTERVAL) => None,
                };

                let current_state = Self::get_current_state(clients_clone.lock().await).await;

                // Ignoring clippy lint here since the `match` provides better clarity.
                #[allow(clippy::option_if_let_else)]
                let has_changed = match &last_state_or {
                    Some(last_state) => &current_state != last_state,
                    // If there was no last state, the state has changed.
                    None => true,
                };

                if has_changed {
                    last_state_or = Some(current_state.clone());

                    // If all receivers have been dropped, stop the task.
                    if view_update_sender.send(current_state).is_err() {
                        break;
                    }
                }

                // If this iteration was triggered by a force update, then send a message
                // back to the caller to indicate that the view is now up to date.
                if let Some(force_update_completed_oneshot) = force_update_completed_oneshot_or {
                    let _ = force_update_completed_oneshot.send(());
                }
            }
        });

        Self {
            derivable_secret: get_derivable_secret(&xprivkey, network),
            clients,
            fedimint_clients_data_dir: project_dirs.data_dir().join(FEDIMINT_CLIENTS_DATA_DIR_NAME),
            view_update_receiver,
            force_update_view_sender,
            view_update_task,
        }
    }

    pub fn get_update_stream(&self) -> tokio_stream::wrappers::WatchStream<WalletView> {
        tokio_stream::wrappers::WatchStream::new(self.view_update_receiver.clone())
    }

    /// Tell `view_update_task` to update the view, and wait for it to complete.
    /// This ensures any streams opened by `get_update_stream`  have yielded the
    /// latest view. This function should be called at the end of any function
    /// that modifies the view.
    ///
    /// Note: This function takes a `MutexGuard` to ensure that the lock isn't
    /// held while waiting for the view to update, which could cause a deadlock.
    async fn force_update_view(
        &self,
        clients: MutexGuard<'_, HashMap<FederationId, ClientHandle>>,
    ) {
        drop(clients);
        let (sender, receiver) = oneshot::channel();
        let _ = self.force_update_view_sender.send(sender).await;
        let _ = receiver.await;
    }

    pub async fn connect_to_joined_federations(&self) -> anyhow::Result<()> {
        // Note: We're intentionally locking the clients mutex earlier than
        // necessary so that the lock is held while we're accessing the data directory.
        let mut clients = self.clients.lock().await;

        // List all files in the data directory.
        let federation_ids = std::fs::read_dir(&self.fedimint_clients_data_dir)?
            .filter_map(|entry| {
                entry.ok().and_then(|entry| {
                    entry
                        .file_name()
                        .into_string()
                        .ok()
                        .and_then(|federation_id| federation_id.parse().ok())
                })
            })
            .collect::<Vec<FederationId>>();

        for federation_id in federation_ids {
            // Skip if we're already connected to this federation.
            if clients.contains_key(&federation_id) {
                continue;
            }

            let db: Database = RocksDb::open(
                self.fedimint_clients_data_dir
                    .join(federation_id.to_string()),
            )?
            .into();

            let client = self
                .build_client_from_federation_id(federation_id, db)
                .await?;

            clients.insert(federation_id, client);
        }

        self.force_update_view(clients).await;

        Ok(())
    }

    pub async fn join_federation(&self, invite_code: InviteCode) -> anyhow::Result<()> {
        // Note: We're intentionally locking the clients mutex earlier than
        // necessary so that the lock is held while we're accessing the data directory.
        let mut clients = self.clients.lock().await;

        let federation_id = invite_code.federation_id();

        let federation_data_dir = self
            .fedimint_clients_data_dir
            .join(federation_id.to_string());

        // Short-circuit if we're already connected to this federation.
        if federation_data_dir.is_dir() {
            return Ok(());
        }

        let db: Database = RocksDb::open(federation_data_dir)?.into();

        let client = self.build_client_from_invite_code(invite_code, db).await?;

        clients.insert(federation_id, client);

        self.force_update_view(clients).await;

        Ok(())
    }

    // TODO: Call `ClientModule::leave()` for every module.
    // https://docs.rs/fedimint-client/0.4.2/fedimint_client/module/trait.ClientModule.html#method.leave
    // Currently it isn't implemented for the `LightningClientModule`, so for now we're just checking
    // that the client has a zero balance.
    pub async fn leave_federation(&self, federation_id: FederationId) -> anyhow::Result<()> {
        // Note: We're intentionally locking the clients mutex earlier than
        // necessary so that the lock is held while we're accessing the data directory.
        let mut clients = self.clients.lock().await;

        if let Some(client) = clients.remove(&federation_id) {
            if client.get_balance().await.msats != 0 {
                // Re-insert the client back into the clients map.
                clients.insert(federation_id, client);

                return Err(anyhow::anyhow!(
                    "Cannot leave federation with non-zero balance: {}",
                    federation_id
                ));
            }

            client.shutdown().await;

            let federation_data_dir = self
                .fedimint_clients_data_dir
                .join(federation_id.to_string());

            if federation_data_dir.is_dir() {
                std::fs::remove_dir_all(federation_data_dir)?;
            }
        }

        self.force_update_view(clients).await;

        Ok(())
    }

    /// Constructs the current view of the wallet.
    /// SHOULD ONLY BE CALLED FROM THE `view_update_task`.
    /// This way, `view_update_task` can only yield values
    /// when the view is changed, with the guarantee that
    /// the view hasn't been updated elsewhere in a way that
    /// could de-sync the view.
    async fn get_current_state(
        clients: MutexGuard<'_, HashMap<FederationId, ClientHandle>>,
    ) -> WalletView {
        let mut federations = BTreeMap::new();

        for (federation_id, client) in clients.iter() {
            let lightning_module = client.get_first_module::<LightningClientModule>();
            let gateways = lightning_module.list_gateways().await;

            federations.insert(
                *federation_id,
                FederationView {
                    federation_id: *federation_id,
                    name_or: client
                        .config()
                        .await
                        .global
                        .federation_name()
                        .map(ToString::to_string),
                    balance: client.get_balance().await,
                    gateways,
                },
            );
        }

        WalletView { federations }
    }

    pub async fn pay_invoice(
        &self,
        invoice: Bolt11Invoice,
        federation_id: FederationId,
    ) -> anyhow::Result<()> {
        let clients = self.clients.lock().await;

        let client = clients
            .get(&federation_id)
            .ok_or_else(|| anyhow::anyhow!("Client for federation {} not found", federation_id))?;

        let lightning_module = client.get_first_module::<LightningClientModule>();

        let gateways = lightning_module.list_gateways().await;

        let payment_info = lightning_module
            .pay_bolt11_invoice(Self::select_gateway(&gateways), invoice, ())
            .await?;

        lightning_module
            .wait_for_ln_payment(payment_info.payment_type, payment_info.contract_id, false)
            .await?;

        self.force_update_view(clients).await;

        Ok(())
    }

    pub async fn receive_payment(
        &self,
        federation_id: FederationId,
        amount: Amount,
        description: String,
    ) -> anyhow::Result<(Bolt11Invoice, oneshot::Receiver<LightningReceiveCompletion>)> {
        let clients = self.clients.lock().await;

        let client = clients
            .get(&federation_id)
            .ok_or_else(|| anyhow::anyhow!("Client for federation {} not found", federation_id))?;

        let lightning_module = client.get_first_module::<LightningClientModule>();

        let gateways = lightning_module.list_gateways().await;

        let (operation_id, invoice, _preimage) = lightning_module
            .create_bolt11_invoice(
                amount,
                Bolt11InvoiceDescription::Direct(&Description::new(description).unwrap()),
                None,
                (),
                Self::select_gateway(gateways.as_slice()),
            )
            .await?;

        let mut update_stream = lightning_module
            .subscribe_ln_receive(operation_id)
            .await?
            .into_stream();

        let (payment_completion_sender, payment_completion_receiver) = oneshot::channel();

        tokio::spawn(async move {
            while let Some(update) = update_stream.next().await {
                match update {
                    LnReceiveState::Claimed => {
                        // If receiver was dropped, we don't care about the result.
                        let _ = payment_completion_sender.send(LightningReceiveCompletion::Success);
                        break;
                    }
                    LnReceiveState::Canceled { .. } => {
                        // If receiver was dropped, we don't care about the result.
                        let _ = payment_completion_sender.send(LightningReceiveCompletion::Failure);
                        break;
                    }
                    _ => {}
                }
            }
        });

        self.force_update_view(clients).await;

        Ok((invoice, payment_completion_receiver))
    }

    async fn build_client_from_invite_code(
        &self,
        invite_code: InviteCode,
        db: Database,
    ) -> anyhow::Result<ClientHandle> {
        let is_initialized = fedimint_client::Client::is_initialized(&db).await;

        let mut client_builder = Client::builder(db).await?;

        // Add lightning and e-cash modules. For now we don't support on-chain.
        client_builder.with_module(fedimint_mint_client::MintClientInit);
        client_builder.with_module(fedimint_ln_client::LightningClientInit::default());

        client_builder.with_primary_module(1);

        let derivable_secret = self.derivable_secret.clone();

        let client = if is_initialized {
            client_builder.open(derivable_secret).await?
        } else {
            let config = fedimint_api_client::download_from_invite_code(&invite_code).await?;

            client_builder
                .join(derivable_secret, config, invite_code.api_secret())
                .await?
        };

        Ok(client)
    }

    async fn build_client_from_federation_id(
        &self,
        federation_id: FederationId,
        db: Database,
    ) -> anyhow::Result<ClientHandle> {
        let is_initialized = fedimint_client::Client::is_initialized(&db).await;

        let mut client_builder = Client::builder(db).await?;

        // Add lightning and e-cash modules. For now we don't support on-chain.
        client_builder.with_module(fedimint_mint_client::MintClientInit);
        client_builder.with_module(fedimint_ln_client::LightningClientInit::default());

        client_builder.with_primary_module(1);

        let derivable_secret = self.derivable_secret.clone();

        let client = if is_initialized {
            client_builder.open(derivable_secret).await?
        } else {
            return Err(anyhow::anyhow!(
                "Federation with ID {} is not initialized.",
                federation_id
            ));
        };

        Ok(client)
    }

    // TODO: Optimize gateway selection algorithm.
    fn select_gateway(gateways: &[LightningGatewayAnnouncement]) -> Option<LightningGateway> {
        let vetted_gateways: Vec<_> = gateways
            .iter()
            .filter(|gateway_announcement| gateway_announcement.vetted)
            .map(|gateway_announcement| &gateway_announcement.info)
            .collect();

        // If there are vetted gateways, select a random one.
        if let Some(random_vetted_gateway) = vetted_gateways.choose(&mut thread_rng()) {
            return Some((*random_vetted_gateway).clone());
        }

        // If there are no vetted gateways, select a random unvetted gateway.
        gateways
            .choose(&mut thread_rng())
            .map(|gateway_announcement| gateway_announcement.info.clone())
    }
}

fn get_derivable_secret(xprivkey: &Xpriv, network: Network) -> DerivableSecret {
    let context = Secp256k1::new();

    let xpriv = xprivkey
        .derive_priv(
            &context,
            &DerivationPath::from(vec![ChildNumber::Normal {
                index: FEDIMINT_DERIVATION_NUMBER,
            }]),
        )
        .expect("This can never fail. Should be fixed in future version of `bitcoin` crate.")
        .derive_priv(
            &context,
            &[
                ChildNumber::from_hardened_idx(coin_type_from_network(network))
                    .expect("Should only fail if 2^31 <= index"),
            ],
        )
        .expect("This can never fail. Should be fixed in future version of `bitcoin` crate.");

    // `Mnemonic::from_entropy()` should only ever fail if the input is not of the correct length.
    // Valid lengths are 128, 160, 192, 224, or 256 bits, and `SecretKey::secret_bytes()` is always 256 bits.
    let mnemonic = Mnemonic::from_entropy(&xpriv.private_key.secret_bytes())
        .expect("Private key should always be 32 bytes");

    Bip39RootSecretStrategy::<12>::to_root_secret(&mnemonic)
}

fn coin_type_from_network(network: Network) -> u32 {
    match network {
        Network::Bitcoin => 0,
        Network::Testnet => 1,
        Network::Signet => 1,
        Network::Regtest => 1,
        net => panic!("Got unknown network: {net}!"),
    }
}
