use std::{fmt::Debug, fs};

use fuel_tx::ConsensusParameters;
use fuel_types::AssetId;
use fuels_core::{
    constants::BASE_ASSET_ID,
    types::{
        bech32::Bech32Address, errors::Result, input::Input,
        transaction_builders::TransactionBuilder, unresolved_bytes::UnresolvedBytes,
    },
    Configurables,
};

use crate::{
    accounts_utils::{adjust_inputs, adjust_outputs, calculate_base_amount_with_fee},
    provider::Provider,
    Account, AccountError, AccountResult, ViewOnlyAccount,
};

#[derive(Debug, Clone)]
pub struct Predicate {
    address: Bech32Address,
    code: Vec<u8>,
    data: UnresolvedBytes,
    provider: Option<Provider>,
}

impl Predicate {
    pub fn address(&self) -> &Bech32Address {
        &self.address
    }

    pub fn code(&self) -> &Vec<u8> {
        &self.code
    }

    pub fn data(&self) -> &UnresolvedBytes {
        &self.data
    }

    pub fn provider(&self) -> Option<&Provider> {
        self.provider.as_ref()
    }

    pub fn set_provider(&mut self, provider: Provider) -> &mut Self {
        self.address = Self::calculate_address(&self.code, &provider.consensus_parameters());
        self.provider = Some(provider);
        self
    }

    pub fn calculate_address(code: &[u8], params: &ConsensusParameters) -> Bech32Address {
        fuel_tx::Input::predicate_owner(code, params).into()
    }

    fn consensus_parameters(&self) -> ConsensusParameters {
        self.provider()
            .map(|p| p.consensus_parameters())
            .unwrap_or_default()
    }

    /// Uses default `ConsensusParameters`
    pub fn from_code(code: Vec<u8>) -> Self {
        Self {
            address: Self::calculate_address(&code, &ConsensusParameters::default()),
            code,
            data: Default::default(),
            provider: None,
        }
    }

    /// Uses default `ConsensusParameters`
    pub fn load_from(file_path: &str) -> Result<Self> {
        let code = fs::read(file_path)?;
        Ok(Self::from_code(code))
    }

    pub fn with_data(mut self, data: UnresolvedBytes) -> Self {
        self.data = data;
        self
    }

    pub fn with_code(self, code: Vec<u8>) -> Self {
        let address = Self::calculate_address(&code, &self.consensus_parameters());
        Self {
            code,
            address,
            ..self
        }
    }

    pub fn with_provider(self, provider: Provider) -> Self {
        let address = Self::calculate_address(&self.code, &provider.consensus_parameters());
        Self {
            address,
            provider: Some(provider),
            ..self
        }
    }

    pub fn with_configurables(mut self, configurables: impl Into<Configurables>) -> Self {
        let configurables: Configurables = configurables.into();
        configurables.update_constants_in(&mut self.code);
        let address = Self::calculate_address(&self.code, &self.consensus_parameters());
        self.address = address;
        self
    }
}

impl ViewOnlyAccount for Predicate {
    fn address(&self) -> &Bech32Address {
        self.address()
    }

    fn try_provider(&self) -> AccountResult<&Provider> {
        self.provider.as_ref().ok_or(AccountError::no_provider())
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Account for Predicate {
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
        let consensus_parameters = self
            .try_provider()?
            .chain_info()
            .await?
            .consensus_parameters;

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
