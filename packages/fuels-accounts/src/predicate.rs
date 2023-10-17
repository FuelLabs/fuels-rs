use std::{fmt::Debug, fs};

use fuel_tx::ConsensusParameters;
use fuel_types::AssetId;
use fuels_core::{
    types::{
        bech32::Bech32Address, errors::Result, input::Input,
        transaction_builders::TransactionBuilder, unresolved_bytes::UnresolvedBytes,
    },
    Configurables,
};

use crate::{provider::Provider, Account, AccountError, AccountResult, ViewOnlyAccount};

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

    pub fn set_provider(&mut self, provider: Provider) {
        self.address = Self::calculate_address(&self.code, provider.chain_id().into());
        self.provider = Some(provider);
    }

    pub fn with_provider(self, provider: Provider) -> Self {
        let address = Self::calculate_address(&self.code, provider.chain_id().into());
        Self {
            address,
            provider: Some(provider),
            ..self
        }
    }

    pub fn calculate_address(code: &[u8], chain_id: u64) -> Bech32Address {
        fuel_tx::Input::predicate_owner(code, &chain_id.into()).into()
    }

    fn consensus_parameters(&self) -> ConsensusParameters {
        self.provider()
            .map(|p| p.consensus_parameters())
            .unwrap_or_default()
    }

    /// Uses default `ConsensusParameters`
    pub fn from_code(code: Vec<u8>) -> Self {
        Self {
            address: Self::calculate_address(&code, ConsensusParameters::default().chain_id.into()),
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
        let address = Self::calculate_address(&code, self.consensus_parameters().chain_id.into());
        Self {
            code,
            address,
            ..self
        }
    }

    pub fn with_configurables(mut self, configurables: impl Into<Configurables>) -> Self {
        let configurables: Configurables = configurables.into();
        configurables.update_constants_in(&mut self.code);
        let address =
            Self::calculate_address(&self.code, self.consensus_parameters().chain_id.into());
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

    fn finalize_tx<Tb: TransactionBuilder>(&self, tb: Tb) -> Result<Tb::TxType> {
        tb.build()
    }
}
