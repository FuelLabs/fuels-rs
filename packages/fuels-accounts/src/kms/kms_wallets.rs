use crate::{
    accounts_utils::try_provider_error,
    kms::{
        aws::AwsKmsSigner,
        google::{CryptoKeyVersionName, GoogleKmsSigner},
    },
    provider::Provider,
    wallet::Wallet,
    Account, ViewOnlyAccount,
};
use async_trait::async_trait;
use aws_sdk_kms::Client as AwsClient;
use fuel_types::AssetId;
use fuels_core::{
    traits::Signer,
    types::{
        bech32::Bech32Address, coin_type_id::CoinTypeId, errors::Result, input::Input,
        transaction_builders::TransactionBuilder,
    },
};
use google_cloud_kms::client::Client as GoogleClient;
use std::fmt::{Debug, Formatter};

pub type AwsWallet = KmsWallet<AwsKmsSigner>;
pub type GoogleWallet = KmsWallet<GoogleKmsSigner>;

pub struct KmsWallet<S: Signer + Send + Sync + Clone> {
    view_account: Wallet,
    kms_signer: S,
}

impl<S: Signer + Send + Sync + Clone> KmsWallet<S> {
    pub fn new(kms_signer: S, provider: Option<Provider>) -> Self {
        Self {
            view_account: Wallet::from_address(kms_signer.address().clone(), provider),
            kms_signer,
        }
    }

    pub fn address(&self) -> &Bech32Address {
        self.kms_signer.address()
    }
    pub fn provider(&self) -> Option<&Provider> {
        self.view_account.provider()
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<S: Signer + Send + Sync + Clone> ViewOnlyAccount for KmsWallet<S> {
    fn address(&self) -> &Bech32Address {
        self.kms_signer.address()
    }
    fn try_provider(&self) -> Result<&Provider> {
        self.provider().ok_or_else(try_provider_error)
    }

    async fn get_asset_inputs_for_amount(
        &self,
        asset_id: AssetId,
        amount: u64,
        excluded_coins: Option<Vec<CoinTypeId>>,
    ) -> Result<Vec<Input>> {
        self.view_account
            .get_asset_inputs_for_amount(asset_id, amount, excluded_coins)
            .await
    }
}

#[async_trait]
impl<S: Signer + Send + Sync + Clone + 'static> Account for KmsWallet<S> {
    fn add_witnesses<Tb: TransactionBuilder>(&self, tb: &mut Tb) -> Result<()> {
        tb.add_signer(self.kms_signer.clone())?;
        Ok(())
    }
}

impl<S: Signer + Send + Sync + Clone> Debug for KmsWallet<S> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KmsWallet")
            .field("address", self.address())
            .field("provider", &self.provider().is_some())
            .finish()
    }
}

impl<S: Signer + Send + Sync + Clone> Clone for KmsWallet<S> {
    fn clone(&self) -> Self {
        Self {
            view_account: self.view_account.clone(),
            kms_signer: self.kms_signer.clone(),
        }
    }
}

impl AwsWallet {
    pub async fn with_kms_key(
        key_id: impl Into<String>,
        aws_client: &AwsClient,
        provider: Option<Provider>,
    ) -> Result<Self> {
        Ok(Self::new(
            AwsKmsSigner::new(key_id.into(), aws_client).await?,
            provider,
        ))
    }
}

impl GoogleWallet {
    pub async fn with_kms_key(
        key_path: impl Into<String>,
        google_client: &GoogleClient,
        provider: Option<Provider>,
    ) -> Result<Self> {
        Ok(Self::new(
            GoogleKmsSigner::new(key_path.into(), google_client).await?,
            provider,
        ))
    }

    pub async fn with_key_version(
        key_name: CryptoKeyVersionName,
        google_client: &GoogleClient,
        provider: Option<Provider>,
    ) -> Result<Self> {
        Self::with_kms_key(key_name.to_string(), google_client, provider).await
    }
}
