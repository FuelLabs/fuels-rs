use url::Url;

use fuel_core_client::client::FuelClient;
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

    pub async fn health(&self) -> Result<bool> {
        match self.client.health().await {
            Ok(healthy) => Ok(healthy),
            Err(err) => Err(Error::Other(err.to_string())),
        }
    }
}
