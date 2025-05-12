use std::{fmt::Debug, fs};

#[cfg(feature = "std")]
use fuels_core::types::{AssetId, coin_type_id::CoinTypeId, input::Input};
use fuels_core::{
    Configurables, error,
    types::{bech32::Bech32Address, errors::Result},
};

#[cfg(feature = "std")]
use crate::accounts_utils::try_provider_error;
#[cfg(feature = "std")]
use crate::{Account, ViewOnlyAccount, provider::Provider};

#[derive(Debug, Clone)]
pub struct Predicate {
    address: Bech32Address,
    code: Vec<u8>,
    data: Vec<u8>,
    #[cfg(feature = "std")]
    provider: Option<Provider>,
}

impl Predicate {
    pub fn address(&self) -> &Bech32Address {
        &self.address
    }

    pub fn code(&self) -> &[u8] {
        &self.code
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn calculate_address(code: &[u8]) -> Bech32Address {
        fuel_tx::Input::predicate_owner(code).into()
    }

    pub fn load_from(file_path: &str) -> Result<Self> {
        let code = fs::read(file_path).map_err(|e| {
            error!(
                IO,
                "could not read predicate binary {file_path:?}. Reason: {e}"
            )
        })?;
        Ok(Self::from_code(code))
    }

    pub fn from_code(code: Vec<u8>) -> Self {
        Self {
            address: Self::calculate_address(&code),
            code,
            data: Default::default(),
            #[cfg(feature = "std")]
            provider: None,
        }
    }

    pub fn with_data(mut self, data: Vec<u8>) -> Self {
        self.data = data;
        self
    }

    pub fn with_code(self, code: Vec<u8>) -> Self {
        let address = Self::calculate_address(&code);
        Self {
            code,
            address,
            ..self
        }
    }

    pub fn with_configurables(mut self, configurables: impl Into<Configurables>) -> Self {
        let configurables: Configurables = configurables.into();
        configurables.update_constants_in(&mut self.code);
        let address = Self::calculate_address(&self.code);
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
        self.provider = Some(provider);
    }

    pub fn with_provider(self, provider: Provider) -> Self {
        Self {
            provider: Some(provider),
            ..self
        }
    }
}

#[cfg(feature = "std")]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl ViewOnlyAccount for Predicate {
    fn address(&self) -> &Bech32Address {
        self.address()
    }

    fn try_provider(&self) -> Result<&Provider> {
        self.provider.as_ref().ok_or_else(try_provider_error)
    }

    async fn get_asset_inputs_for_amount(
        &self,
        asset_id: AssetId,
        amount: u128,
        excluded_coins: Option<Vec<CoinTypeId>>,
    ) -> Result<Vec<Input>> {
        Ok(self
            .get_spendable_resources(asset_id, amount, excluded_coins)
            .await?
            .into_iter()
            .map(|resource| {
                Input::resource_predicate(resource, self.code.clone(), self.data.clone())
            })
            .collect::<Vec<Input>>())
    }
}

#[cfg(feature = "std")]
impl Account for Predicate {}
