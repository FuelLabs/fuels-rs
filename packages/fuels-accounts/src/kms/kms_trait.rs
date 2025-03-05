use crate::accounts_utils::try_provider_error;
use crate::provider::Provider;
use crate::wallet::Wallet;
use crate::{Account, ViewOnlyAccount};
use async_trait::async_trait;
use fuel_crypto::{Message, Signature};
use fuel_types::AssetId;
use fuels_core::{
    traits::Signer,
    types::{
        bech32::Bech32Address, coin_type_id::CoinTypeId, errors::Result, input::Input,
        transaction_builders::TransactionBuilder,
    },
};
use std::fmt::Debug;

#[async_trait]
pub trait KmsSigner: Clone + Send + Sync + Debug {
    fn fuel_address(&self) -> &Bech32Address;

    async fn sign_message(&self, message: Message) -> Result<Signature>;
}

#[derive(Clone, Debug)]
pub struct KmsWallet<S: KmsSigner> {
    view_account: Wallet,
    kms_signer: S,
}

impl<S: KmsSigner> KmsWallet<S> {
    pub fn new(kms_signer: S, provider: Option<Provider>) -> Self {
        Self {
            view_account: Wallet::from_address(kms_signer.fuel_address().clone(), provider),
            kms_signer,
        }
    }

    pub fn address(&self) -> &Bech32Address {
        self.kms_signer.fuel_address()
    }

    pub fn provider(&self) -> Option<&Provider> {
        self.view_account.provider()
    }
}

#[async_trait]
impl<S: KmsSigner + 'static> Signer for KmsWallet<S> {
    async fn sign(&self, message: Message) -> Result<Signature> {
        self.kms_signer.sign_message(message).await
    }

    fn address(&self) -> &Bech32Address {
        self.address()
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<S: KmsSigner> ViewOnlyAccount for KmsWallet<S> {
    fn address(&self) -> &Bech32Address {
        self.kms_signer.fuel_address()
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
impl<S: KmsSigner + 'static> Account for KmsWallet<S> {
    fn add_witnesses<Tb: TransactionBuilder>(&self, tb: &mut Tb) -> Result<()> {
        tb.add_signer(self.clone())?;
        Ok(())
    }
}
