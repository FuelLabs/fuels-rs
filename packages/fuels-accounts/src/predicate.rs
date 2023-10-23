use std::{fmt::Debug, fs};

#[cfg(feature = "std")]
use fuels_core::{
    constants::BASE_ASSET_ID,
    types::{input::Input, transaction_builders::TransactionBuilder, AssetId},
};
use fuels_core::{
    types::{bech32::Bech32Address, errors::Result, unresolved_bytes::UnresolvedBytes},
    Configurables,
};

#[cfg(feature = "std")]
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
    chain_id: u64,
    #[cfg(feature = "std")]
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

    pub fn calculate_address(code: &[u8], chain_id: u64) -> Bech32Address {
        fuel_tx::Input::predicate_owner(code, &chain_id.into()).into()
    }

    /// Uses default `ChainId`
    pub fn load_from(file_path: &str) -> Result<Self> {
        let code = fs::read(file_path)?;
        Ok(Self::from_code(code, 0))
    }

    pub fn from_code(code: Vec<u8>, chain_id: u64) -> Self {
        Self {
            address: Self::calculate_address(&code, chain_id),
            chain_id,
            code,
            data: Default::default(),
            #[cfg(feature = "std")]
            provider: None,
        }
    }

    pub fn with_data(mut self, data: UnresolvedBytes) -> Self {
        self.data = data;
        self
    }

    pub fn with_code(self, code: Vec<u8>) -> Self {
        let address = Self::calculate_address(&code, self.chain_id);
        Self {
            code,
            address,
            ..self
        }
    }

    pub fn with_configurables(mut self, configurables: impl Into<Configurables>) -> Self {
        let configurables: Configurables = configurables.into();
        configurables.update_constants_in(&mut self.code);
        let address = Self::calculate_address(&self.code, self.chain_id);
        self.address = address;
        self
    }
}

#[cfg(feature = "std")]
impl Predicate {
    pub fn provider(&self) -> Option<&Provider> {
        self.provider.as_ref()
    }

    pub fn set_provider(&mut self, provider: Provider) {
        self.address = Self::calculate_address(&self.code, provider.chain_id().into());
        self.chain_id = provider.chain_id().into();
        self.provider = Some(provider);
    }

    pub fn with_provider(self, provider: Provider) -> Self {
        let address = Self::calculate_address(&self.code, provider.chain_id().into());
        Self {
            address,
            chain_id: provider.chain_id().into(),
            provider: Some(provider),
            ..self
        }
    }
}

#[cfg(feature = "std")]
impl ViewOnlyAccount for Predicate {
    fn address(&self) -> &Bech32Address {
        self.address()
    }

    fn try_provider(&self) -> AccountResult<&Provider> {
        self.provider.as_ref().ok_or(AccountError::no_provider())
    }
}

#[cfg(feature = "std")]
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
    ) -> Result<Tb::TxType> {
        let network_info = self.try_provider()?.network_info().await?;
        let new_base_amount =
            calculate_base_amount_with_fee(&tb, &network_info, previous_base_amount)?;

        let new_base_inputs = self
            .get_asset_inputs_for_amount(BASE_ASSET_ID, new_base_amount)
            .await?;

        adjust_inputs(&mut tb, new_base_inputs);
        adjust_outputs(&mut tb, self.address(), new_base_amount);

        tb.build()
    }
}
