use fuels_core::types::errors::{Error, Result};
use google_cloud_kms::client::Client;
pub use google_cloud_kms::client::{google_cloud_auth::credentials::CredentialsFile, ClientConfig};

#[derive(Clone, Debug)]
pub struct GoogleClient {
    client: Client,
}

impl GoogleClient {
    pub async fn new(config: ClientConfig) -> Result<Self> {
        let config_debug = format!("{:?}", config);

        let client = Client::new(config).await.map_err(|e| {
            Error::Other(format!(
                "Failed to create Google KMS client with config {}: {}",
                config_debug, e
            ))
        })?;

        Ok(Self { client })
    }

    pub fn inner(&self) -> &Client {
        &self.client
    }
}
