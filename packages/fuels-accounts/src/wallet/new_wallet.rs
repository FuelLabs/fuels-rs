use async_trait::async_trait;
use fuel_tx::AssetId;
use fuels_core::{
    traits::Signer,
    types::{
        bech32::Bech32Address, coin_type_id::CoinTypeId, errors::Result, input::Input,
        transaction_builders::TransactionBuilder,
    },
};

use crate::{provider::Provider, signers::PrivateKeySigner, Account, ViewOnlyAccount};

#[derive(Debug, Clone)]
pub struct NewWallet<S = PrivateKeySigner> {
    signer: S,
    provider: Provider,
}

impl<S> NewWallet<S> {
    pub fn new(signer: S, provider: Provider) -> Self {
        Self { signer, provider }
    }
}

#[async_trait]
impl<S> ViewOnlyAccount for NewWallet<S>
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
impl<S> Account for NewWallet<S>
where
    S: Signer + Clone + Send + Sync + std::fmt::Debug,
{
    fn add_witnesses<Tb: TransactionBuilder>(&self, tb: &mut Tb) -> Result<()> {
        tb.add_signer(self.signer.clone())?;

        Ok(())
    }
}
