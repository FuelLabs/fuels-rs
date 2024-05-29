use std::{collections::HashMap, fmt::Debug};

use fuel_tx::{AssetId, Output};
use fuels_core::types::{
    bech32::{Bech32Address, Bech32ContractId},
    errors::Result,
    param_types::ParamType,
    Selector,
};

use crate::{calls::utils::sealed, contract::CallParameters};

#[derive(Debug, Clone)]
/// Contains all data relevant to a single contract call
pub struct ContractCall {
    pub contract_id: Bech32ContractId,
    pub encoded_args: Result<Vec<u8>>,
    pub encoded_selector: Selector,
    pub call_parameters: CallParameters,
    pub variable_outputs: Vec<Output>,
    pub external_contracts: Vec<Bech32ContractId>,
    pub output_param: ParamType,
    pub is_payable: bool,
    pub custom_assets: HashMap<(AssetId, Option<Bech32Address>), u64>,
}

impl ContractCall {
    pub fn with_contract_id(self, contract_id: Bech32ContractId) -> Self {
        ContractCall {
            contract_id,
            ..self
        }
    }

    pub fn with_variable_outputs(self, variable_outputs: Vec<Output>) -> ContractCall {
        ContractCall {
            variable_outputs,
            ..self
        }
    }

    pub fn with_call_parameters(self, call_parameters: CallParameters) -> ContractCall {
        ContractCall {
            call_parameters,
            ..self
        }
    }

    pub fn add_custom_asset(&mut self, asset_id: AssetId, amount: u64, to: Option<Bech32Address>) {
        *self.custom_assets.entry((asset_id, to)).or_default() += amount;
    }
}

impl sealed::Sealed for ContractCall {}
