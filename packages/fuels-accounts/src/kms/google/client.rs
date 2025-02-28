use google_cloud_kms::client::google_cloud_auth::credentials::CredentialsFile;
use google_cloud_kms::client::Client;
use google_cloud_kms::client::ClientConfig;

use fuels_core::types::errors::{Error, Result};

#[derive(Clone)]
pub struct GoogleConfig {
    project_id: String,
    location: String,
    credentials_file: Option<String>,
}

impl GoogleConfig {
    pub fn new(project_id: String, location: String, credentials_file: Option<String>) -> Self {
        Self {
            project_id,
            location,
            credentials_file,
        }
    }

    pub fn from_environment() -> Self {
        let project_id = std::env::var("GOOGLE_CLOUD_PROJECT")
            .expect("GOOGLE_CLOUD_PROJECT environment variable not set");
        let location =
            std::env::var("GOOGLE_CLOUD_LOCATION").unwrap_or_else(|_| "global".to_string());
        let credentials_file = std::env::var("GOOGLE_APPLICATION_CREDENTIALS").ok();

        Self {
            project_id,
            location,
            credentials_file,
        }
    }

    #[cfg(feature = "test-helpers")]
    pub fn for_testing(
        project_id: String,
        location: String,
        credentials_file: Option<String>,
    ) -> Self {
        Self {
            project_id,
            location,
            credentials_file,
        }
    }

    pub fn project_id(&self) -> &str {
        &self.project_id
    }

    pub fn location(&self) -> &str {
        &self.location
    }

    pub fn credentials_file(&self) -> Option<&str> {
        self.credentials_file.as_deref()
    }
}

#[derive(Clone, Debug)]
pub struct GoogleClient {
    client: Client,
}

impl GoogleClient {
    pub async fn new(config: GoogleConfig) -> Result<Self> {
        let mut client_config = ClientConfig::default();

        if let Some(creds_file) = &config.credentials_file {
            let creds = CredentialsFile::new_from_file(creds_file.clone())
                .await
                .map_err(|e| {
                    Error::Other(format!(
                        "Failed to read credentials file {}: {}",
                        creds_file, e
                    ))
                })?;

            client_config = client_config.with_credentials(creds).await.map_err(|e| {
                Error::Other(format!(
                    "Failed to create Google KMS client with credentials file {}: {}",
                    creds_file, e
                ))
            })?;
        }

        let config_debug = format!("{:?}", client_config);
        let client = Client::new(client_config).await.map_err(|e| {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_kms() {}
}
