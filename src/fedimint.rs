use std::{
    collections::{BTreeMap, HashMap},
    path::PathBuf,
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
use iced::futures::{lock::Mutex, StreamExt};
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

const FEDIMINT_CLIENTS_DATA_DIR_NAME: &str = "fedimint_clients";
// TODO: Figure out if we even want this. If we do, it probably shouldn't live here.
// It'd make more sense for it to live wherever the key is maintained elsewhere, and
// have `Wallet::new()` assume that the key is already derived.
const FEDIMINT_DERIVATION_NUMBER: u32 = 1;

pub enum LightningReceiveCompletion {
    Success,
    Failure,
}

#[derive(Debug, Clone)]
pub struct FederationView {
    pub name_or: Option<String>,
    pub balance: Amount,
    pub gateways: Vec<LightningGatewayAnnouncement>,
}

pub struct Wallet {
    derivable_secret: DerivableSecret,
    clients: Mutex<HashMap<FederationId, ClientHandle>>,
    fedimint_clients_data_dir: PathBuf,
}

impl Wallet {
    pub fn new(xprivkey: Xpriv, network: Network, project_dirs: &ProjectDirs) -> Self {
        Self {
            derivable_secret: get_derivable_secret(&xprivkey, network),
            clients: Mutex::new(HashMap::new()),
            fedimint_clients_data_dir: project_dirs.data_dir().join(FEDIMINT_CLIENTS_DATA_DIR_NAME),
        }
    }

    pub async fn connect_to_joined_federations(&self) -> anyhow::Result<()> {
        // Note: We're intentionally locking the clients mutex earlier than
        // necessary so that the lock is held while we're reading the data directory.
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

        Ok(())
    }

    pub async fn join_federation(&self, invite_code: InviteCode) -> anyhow::Result<()> {
        // Note: We're intentionally locking the clients mutex earlier than
        // necessary so that the lock is held while we're reading the data directory.
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

        Ok(())
    }

    pub async fn get_current_state(&self) -> BTreeMap<FederationId, FederationView> {
        let mut state = BTreeMap::new();

        for (federation_id, client) in self.clients.lock().await.iter() {
            let lightning_module = client.get_first_module::<LightningClientModule>();
            let gateways = lightning_module.list_gateways().await;

            state.insert(
                *federation_id,
                FederationView {
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

        state
    }

    pub async fn receive_payment(
        &self,
        federation_id: FederationId,
        amount: Amount,
        description: String,
    ) -> anyhow::Result<(
        Bolt11Invoice,
        iced::futures::channel::oneshot::Receiver<LightningReceiveCompletion>,
    )> {
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

        let (payment_completion_sender, payment_completion_receiver) =
            iced::futures::channel::oneshot::channel();

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
