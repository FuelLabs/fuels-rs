use std::fmt::Debug;

use fuel_types::AssetId;
use fuels_types::{
    bech32::Bech32Address, constants::BASE_ASSET_ID, errors::Result, input::Input,
    transaction_builders::TransactionBuilder, unresolved_bytes::UnresolvedBytes,
};

use crate::{
    accounts_utils::{adjust_inputs, adjust_outputs, calculate_base_amount_with_fee},
    provider::Provider,
    Account, AccountError, AccountResult,
};

#[derive(Debug, Clone)]
pub struct Predicate {
    pub address: Bech32Address,
    pub code: Vec<u8>,
    pub data: UnresolvedBytes,
    pub provider: Option<Provider>,
}

type PredicateResult<T> = std::result::Result<T, AccountError>;

impl Predicate {
    pub fn provider(&self) -> PredicateResult<&Provider> {
        self.provider.as_ref().ok_or(AccountError::NoProvider)
    }

    pub fn set_provider(&mut self, provider: Provider) {
        self.provider = Some(provider)
    }

    pub fn address(&self) -> &Bech32Address {
        &self.address
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Account for Predicate {
    fn address(&self) -> &Bech32Address {
        &self.address
    }

    fn provider(&self) -> AccountResult<&Provider> {
        self.provider.as_ref().ok_or(AccountError::NoProvider)
    }

    fn set_provider(&mut self, provider: Provider) {
        self.set_provider(provider)
    }

    async fn get_asset_inputs_for_amount(
        &self,
        asset_id: AssetId,
        amount: u64,
        _witness_index: Option<u8>,
    ) -> Result<Vec<Input>> {
        Ok(self
            .get_spendable_resources(asset_id, amount)
            .await?
            .into_iter()
            .map(|resource| {
                Input::resource_predicate(resource, self.code.clone(), self.data.clone())
            })
            .collect::<Vec<Input>>())
    }

    /// Add base asset inputs to the transaction to cover the estimated fee.
    /// The original base asset amount cannot be calculated reliably from
    /// the existing transaction inputs because the selected resources may exceed
    /// the required amount to avoid dust. Therefore we require it as an argument.
    ///
    /// Requires contract inputs to be at the start of the transactions inputs vec
    /// so that their indexes are retained
    async fn add_fee_resources<Tb: TransactionBuilder>(
        &self,
        mut tb: Tb,
        previous_base_amount: u64,
        _witness_index: Option<u8>,
    ) -> Result<Tb::TxType> {
        let consensus_parameters = self.provider()?.chain_info().await?.consensus_parameters;

        tb = tb.set_consensus_parameters(consensus_parameters);

        let new_base_amount =
            calculate_base_amount_with_fee(&tb, &consensus_parameters, previous_base_amount);

        let new_base_inputs = self
            .get_asset_inputs_for_amount(BASE_ASSET_ID, new_base_amount, None)
            .await?;

        adjust_inputs(&mut tb, new_base_inputs);
        adjust_outputs(&mut tb, self.address(), new_base_amount);

        let tx = tb.build()?;

        Ok(tx)
    }
}
