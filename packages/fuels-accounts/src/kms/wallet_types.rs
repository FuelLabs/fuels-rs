use crate::kms::aws::AwsKmsSigner;
use crate::kms::google::{CryptoKeyVersionName, GoogleKmsSigner};
use crate::kms::kms_trait::KmsWallet;
use crate::provider::Provider;
use aws_sdk_kms::Client as AwsClient;
use fuels_core::types::errors::Result;
use google_cloud_kms::client::Client as GoogleClient;

pub type AwsWallet = KmsWallet<AwsKmsSigner>;

pub type GoogleWallet = KmsWallet<GoogleKmsSigner>;

impl AwsWallet {
    pub async fn with_kms_key(
        key_id: impl Into<String>,
        aws_client: &AwsClient,
        provider: Option<Provider>,
    ) -> Result<Self> {
        let signer = AwsKmsSigner::new(key_id.into(), aws_client).await?;

        Ok(Self::new(signer, provider))
    }
}

impl GoogleWallet {
    pub async fn with_kms_key(
        key_path: impl Into<String>,
        google_client: &GoogleClient,
        provider: Option<Provider>,
    ) -> Result<Self> {
        let signer = GoogleKmsSigner::new(key_path.into(), google_client).await?;

        Ok(Self::new(signer, provider))
    }

    pub async fn with_key_version(
        key_name: CryptoKeyVersionName,
        google_client: &GoogleClient,
        provider: Option<Provider>,
    ) -> Result<Self> {
        let key_path = key_name.to_string();

        Self::with_kms_key(key_path, google_client, provider).await
    }
}
