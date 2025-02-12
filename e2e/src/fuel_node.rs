use crate::client::HttpClient;
use anyhow::Context;
use fuel_core_types::{
    fuel_tx::AssetId,
};
use fuels::accounts::Account;
use fuels::crypto::SecretKey;
use fuels::prelude::{Bech32Address, Provider, TxPolicies, WalletUnlocked};
use std::str::FromStr;
use url::Url;

#[derive(Default, Debug)]
pub struct FuelNode {
    show_logs: bool,
}

pub struct FuelNodeProcess {
    _child: tokio::process::Child,
    url: Url,
}

impl FuelNode {
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

    pub async fn fund(&self, address: Bech32Address, amount: u64) -> anyhow::Result<()> {
        let fuels_provider = Provider::connect(self.url()).await?;

        // Create a wallet with the private key of the default account
        let mut default_wallet = WalletUnlocked::new_from_private_key(
            SecretKey::from_str(
                "0xde97d8624a438121b86a1956544bd72ed68cd69f2c99555b08b1e8c51ffd511c",
            )?,
            None,
        );
        default_wallet.set_provider(fuels_provider.clone());

        // Transfer ETH funds to the AWS wallet from the default wallet
        let asset_id =
            AssetId::from_str("f8f8b6283d7fa5b672b530cbb84fcccb4ff8dc40f8176ef4544ddb1f1952ad07")
                .expect("AssetId to be well formed");

        default_wallet
            .transfer(&address, amount, asset_id, TxPolicies::default())
            .await
            .context("Failed to transfer funds")?;

        self.client().produce_blocks(1).await?;

        Ok(())
    }
}
