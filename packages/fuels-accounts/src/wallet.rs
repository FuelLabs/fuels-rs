use async_trait::async_trait;
use fuel_tx::AssetId;
use fuels_core::{
    traits::Signer,
    types::{
        bech32::Bech32Address, coin_type_id::CoinTypeId, errors::Result, input::Input,
        transaction_builders::TransactionBuilder,
    },
};
use rand::{CryptoRng, RngCore};

use crate::{
    provider::Provider,
    signers::{FakeSigner, PrivateKeySigner},
    Account, ViewOnlyAccount,
};

#[derive(Debug, Clone)]
pub struct Wallet<S = PrivateKeySigner> {
    signer: S,
    provider: Provider,
}

impl<S> Wallet<S> {
    pub fn new(signer: S, provider: Provider) -> Self {
        Self { signer, provider }
    }

    pub fn provider(&self) -> &Provider {
        &self.provider
    }

    pub fn signer(&self) -> &S {
        &self.signer
    }
}

impl Wallet<PrivateKeySigner> {
    pub fn random(rng: &mut (impl CryptoRng + RngCore), provider: Provider) -> Self {
        Self::new(PrivateKeySigner::random(rng), provider)
    }
}

impl<S> Wallet<S>
where
    S: Signer,
{
    pub fn locked(&self) -> Wallet<FakeSigner> {
        Wallet::new(
            FakeSigner::new(self.signer.address().clone()),
            self.provider.clone(),
        )
    }
}

#[async_trait]
impl<S> ViewOnlyAccount for Wallet<S>
where
    S: Signer + Clone + Send + Sync + std::fmt::Debug,
{
    fn address(&self) -> &Bech32Address {
        self.signer.address()
    }

    fn try_provider(&self) -> Result<&Provider> {
        Ok(&self.provider)
    }

    async fn get_asset_inputs_for_amount(
        &self,
        asset_id: AssetId,
        amount: u64,
        excluded_coins: Option<Vec<CoinTypeId>>,
    ) -> Result<Vec<Input>> {
        Ok(self
            .get_spendable_resources(asset_id, amount, excluded_coins)
            .await?
            .into_iter()
            .map(Input::resource_signed)
            .collect::<Vec<Input>>())
    }
}

#[async_trait]
impl<S> Account for Wallet<S>
where
    S: Signer + Clone + Send + Sync + std::fmt::Debug + 'static,
{
    fn add_witnesses<Tb: TransactionBuilder>(&self, tb: &mut Tb) -> Result<()> {
        tb.add_signer(self.signer.clone())?;

        Ok(())
    }
}
