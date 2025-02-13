use fuels_core::types::errors::{Error, Result};
use google_cloud_auth::credentials::CredentialsFile;
use google_cloud_kms::client::{Client, ClientConfig};
use std::env;
#[derive(Debug)]
pub struct GcpKmsConfig {
    client_config: ClientConfig,
}

impl GcpKmsConfig {
    pub async fn from_environment() -> Result<Self> {
        let credentials_path = env::var("GOOGLE_APPLICATION_CREDENTIALS")
            .map_err(|_| Error::Other("GOOGLE_APPLICATION_CREDENTIALS not set".to_string()))?;

        let credentials = CredentialsFile::new_from_file(credentials_path)
            .await
            .map_err(|e| Error::Other(format!("Failed to load Google KMS credentials: {}", e)))?;

        let client_config = ClientConfig::default()
            .with_credentials(credentials)
            .await
            .map_err(|e| Error::Other(format!("Failed to create Google KMS client: {}", e)))?;
        Ok(Self { client_config })
    }
}

#[derive(Clone, Debug)]
pub struct GcpKmsClient {
    inner: Client,
}

impl GcpKmsClient {
    pub async fn new(config: GcpKmsConfig) -> Result<Self> {
        let client = Client::new(config.client_config)
            .await
            .map_err(|e| Error::Other(format!("Failed to create Google KMS client: {}", e)))?;

        Ok(Self { inner: client })
    }

    pub fn inner(&self) -> &Client {
        &self.inner
    }
}
