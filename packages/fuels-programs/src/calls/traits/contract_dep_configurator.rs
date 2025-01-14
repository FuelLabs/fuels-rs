use fuels_core::types::bech32::Bech32ContractId;

use crate::calls::{utils::sealed, ContractCall, ScriptCall};

pub trait ContractDependencyConfigurator: sealed::Sealed {
    fn append_external_contract(&mut self, contract_id: Bech32ContractId);
    fn with_external_contracts(self, external_contracts: Vec<Bech32ContractId>) -> Self;
}

impl ContractDependencyConfigurator for ContractCall {
    fn append_external_contract(&mut self, contract_id: Bech32ContractId) {
        self.external_contracts.push(contract_id)
    }

    fn with_external_contracts(self, external_contracts: Vec<Bech32ContractId>) -> Self {
        ContractCall {
            external_contracts,
            ..self
        }
    }
}

impl ContractDependencyConfigurator for ScriptCall {
    fn append_external_contract(&mut self, contract_id: Bech32ContractId) {
        self.external_contracts.push(contract_id)
    }

    fn with_external_contracts(self, external_contracts: Vec<Bech32ContractId>) -> Self {
        ScriptCall {
            external_contracts,
            ..self
        }
    }
}
