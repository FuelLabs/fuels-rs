use url::Url;

use fuel_core_client::client::types::CoinType;
use fuel_core_client::client::{types::Block, FuelClient};
use fuel_core_types::fuel_tx::Transaction;
use fuel_core_types::fuel_types::{Address, AssetId};
use fuels::types::coin::Coin;
use fuels::types::errors::Error;
use fuels::types::errors::Result;
#[derive(Clone)]
pub struct HttpClient {
    client: FuelClient,
}

impl HttpClient {
    #[must_use]
    pub fn new(url: &Url) -> Self {
        let client = FuelClient::new(url).expect("Url to be well formed");
        Self { client }
    }

    pub async fn produce_blocks(&self, num: u32) -> Result<()> {
        self.client
            .produce_blocks(num, None)
            .await
            .map_err(|e| Error::Other(e.to_string()))?;

        Ok(())
    }

    pub async fn send_tx(&self, tx: &Transaction) -> Result<()> {
        self.client
            .submit_and_await_commit(tx)
            .await
            .map_err(|e| Error::Other(e.to_string()))?;

        Ok(())
    }

    pub async fn get_coin(&self, address: Address, asset_id: AssetId) -> Result<Coin> {
        let coin_type = self
            .client
            .coins_to_spend(&address, vec![(asset_id, 1, None)], None)
            .await
            .map_err(|e| Error::Other(e.to_string()))?[0][0];

        let coin = match coin_type {
            CoinType::Coin(c) => Ok(c),
            _ => Err(Error::Other("Couldn't get coin".to_string())),
        }?;

        Ok(Coin::from(coin))
    }

    pub async fn health(&self) -> Result<bool> {
        match self.client.health().await {
            Ok(healthy) => Ok(healthy),
            Err(err) => Err(Error::Other(err.to_string())),
        }
    }
}
