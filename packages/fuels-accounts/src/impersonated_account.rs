use std::fmt::Debug;

use async_trait::async_trait;
use fuel_crypto::{Message, Signature};
use fuels_core::traits::Signer;
use fuels_core::types::transaction_builders::TransactionBuilder;
use fuels_core::types::{bech32::Bech32Address, errors::Result};
use fuels_core::types::{coin_type_id::CoinTypeId, input::Input, AssetId};

use crate::accounts_utils::try_provider_error;
use crate::{provider::Provider, Account, ViewOnlyAccount};

/// A `ImpersonatedAccount` simulates ownership of assets held by an account with a given address.
/// `ImpersonatedAccount` will only succeed in unlocking assets if the the network is setup with
/// utxo_validation set to false.
#[derive(Debug, Clone)]
pub struct ImpersonatedAccount {
    address: Bech32Address,
    provider: Option<Provider>,
}

impl ImpersonatedAccount {
    pub fn new(address: Bech32Address, provider: Option<Provider>) -> Self {
        Self { address, provider }
    }

    pub fn address(&self) -> &Bech32Address {
        &self.address
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl ViewOnlyAccount for ImpersonatedAccount {
    fn address(&self) -> &Bech32Address {
        self.address()
    }

    fn try_provider(&self) -> Result<&Provider> {
        self.provider.as_ref().ok_or_else(try_provider_error)
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

impl Account for ImpersonatedAccount {
    fn add_witnesses<Tb: TransactionBuilder>(&self, tb: &mut Tb) -> Result<()> {
        tb.add_signer(self.clone())?;

        Ok(())
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Signer for ImpersonatedAccount {
    async fn sign(&self, _message: Message) -> Result<Signature> {
        Ok(Signature::default())
    }

    fn address(&self) -> &Bech32Address {
        &self.address
    }
}
