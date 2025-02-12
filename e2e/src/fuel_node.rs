use crate::client::HttpClient;
use fuel_core_chain_config::{
    ChainConfig, CoinConfig, ConsensusConfig, SnapshotWriter, StateConfig,
};
use fuel_core_types::{
    fuel_crypto::SecretKey as FuelSecretKey,
    fuel_tx::{AssetId, Finalizable, Input, Output, TransactionBuilder, TxPointer},
    fuel_types::Address,
};
use fuels::crypto::{PublicKey, SecretKey};
use fuels::prelude::{Provider, WalletUnlocked};
use itertools::Itertools;
use rand::Rng;
use std::path::PathBuf;
use std::str::FromStr;
use url::Url;
use fuels::accounts::Account;
use fuels::accounts::aws_signer::AwsWallet;
use fuels::types::U256;

#[derive(Default, Debug)]
pub struct FuelNode {
    show_logs: bool,
}

pub struct FuelNodeProcess {
    _child: tokio::process::Child,
    url: Url,
}

impl FuelNode {
    fn create_state_config(
        path: impl Into<PathBuf>,
        consensus_key: &PublicKey,
        num_wallets: usize,
    ) -> anyhow::Result<Vec<FuelSecretKey>> {
        let chain_config = ChainConfig {
            consensus: ConsensusConfig::PoA {
                signing_key: Input::owner(consensus_key),
            },
            ..ChainConfig::local_testnet()
        };

        let mut rng = &mut rand::thread_rng();
        let keys = std::iter::repeat_with(|| FuelSecretKey::random(&mut rng))
            .take(num_wallets)
            .collect_vec();

        let coins = keys
            .iter()
            .flat_map(|key| {
                std::iter::repeat_with(|| CoinConfig {
                    owner: Input::owner(&key.public_key()),
                    amount: u64::MAX,
                    asset_id: AssetId::zeroed(),
                    tx_id: rng.gen(),
                    output_index: rng.gen(),
                    ..Default::default()
                })
                .take(10)
                .collect_vec()
            })
            .collect_vec();

        let state_config = StateConfig {
            coins,
            ..StateConfig::local_testnet()
        };

        let snapshot = SnapshotWriter::json(path);
        snapshot
            .write_state_config(state_config, &chain_config)
            .map_err(|_| anyhow::anyhow!("Failed to write state config"))?;

        Ok(keys)
    }

    pub async fn start(&self) -> anyhow::Result<FuelNodeProcess> {
        let unused_port = portpicker::pick_unused_port()
            .ok_or_else(|| anyhow::anyhow!("No free port to start fuel-core"))?;

        let mut cmd = tokio::process::Command::new("fuel-core");

        cmd.arg("run")
            .arg("--port")
            .arg(unused_port.to_string())
            .arg("--db-type")
            .arg("in-memory")
            .arg("--debug")
            .kill_on_drop(true)
            .stdin(std::process::Stdio::null());

        let sink = if self.show_logs {
            std::process::Stdio::inherit
        } else {
            std::process::Stdio::null
        };
        cmd.stdout(sink()).stderr(sink());

        let child = cmd.spawn()?;

        let url = format!("http://localhost:{}", unused_port).parse()?;

        let process = FuelNodeProcess { _child: child, url };

        process.wait_until_healthy().await;

        Ok(process)
    }

    pub fn with_show_logs(mut self, show_logs: bool) -> Self {
        self.show_logs = show_logs;
        self
    }
}

impl FuelNodeProcess {
    pub fn client(&self) -> HttpClient {
        HttpClient::new(&self.url)
    }

    async fn send_transfer_tx(client: HttpClient, key: FuelSecretKey) -> anyhow::Result<()> {
        let mut tx = TransactionBuilder::script(vec![], vec![]);

        tx.script_gas_limit(1_000_000);

        let secret_key = key;
        let address = Input::owner(&secret_key.public_key());

        let base_asset = AssetId::zeroed();
        let coin = client.get_coin(address, base_asset).await?;

        tx.add_unsigned_coin_input(
            secret_key,
            coin.utxo_id,
            coin.amount,
            coin.asset_id,
            TxPointer::default(),
        );

        const AMOUNT: u64 = 1;
        let to = Address::default();
        tx.add_output(Output::Coin {
            to,
            amount: AMOUNT,
            asset_id: base_asset,
        });
        tx.add_output(Output::Change {
            to: address,
            amount: 0,
            asset_id: base_asset,
        });

        let tx = tx.finalize();

        client.send_tx(&tx.into()).await?;

        Ok(())
    }

    async fn wait_until_healthy(&self) {
        loop {
            if let Ok(true) = self.client().health().await {
                break;
            }
        }
    }

    pub fn url(&self) -> &Url {
        &self.url
    }

    pub async fn fund(
        &self,
        address: Address,
        amount: U256
    ) -> anyhow::Result<()> {
        let fuels_provider = Provider::connect(self.url()).await.unwrap();

        let mut default_wallet = WalletUnlocked::new_from_private_key(
            SecretKey::from_str(
                "0xde97d8624a438121b86a1956544bd72ed68cd69f2c99555b08b1e8c51ffd511c",
            )
            .unwrap(),
            None,
        );

        default_wallet.set_provider(fuels_provider.clone());
        // let wallet = AwsWallet::from_kms_key_id(key.id, provider).await?;

        // default_wallet
        //     .transfer(
        //         wallet.address(),
        //         1_000_000_000, // Amount to transfer
        //         AssetId::default(), // Using the base asset
        //         TxParams::default(),
        //     )
        //     .await?;
        // if succeeded {
        //     Ok(())
        // } else {
        //     Err(anyhow::anyhow!("Failed to fund address {address}"))
        // }
        Ok(())
    }
}
