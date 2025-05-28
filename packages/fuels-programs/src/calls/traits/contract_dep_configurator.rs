use fuels_core::types::ContractId;

use crate::calls::{ContractCall, ScriptCall, utils::sealed};

pub trait ContractDependencyConfigurator: sealed::Sealed {
    fn append_external_contract(&mut self, contract_id: ContractId);
    fn with_external_contracts(self, external_contracts: Vec<ContractId>) -> Self;
}

impl ContractDependencyConfigurator for ContractCall {
    fn append_external_contract(&mut self, contract_id: ContractId) {
        self.external_contracts.push(contract_id)
    }

    fn with_external_contracts(self, external_contracts: Vec<ContractId>) -> Self {
        ContractCall {
            external_contracts,
            ..self
        }
    }
}

impl ContractDependencyConfigurator for ScriptCall {
    fn append_external_contract(&mut self, contract_id: ContractId) {
        self.external_contracts.push(contract_id)
    }

    fn with_external_contracts(self, external_contracts: Vec<ContractId>) -> Self {
        ScriptCall {
            external_contracts,
            ..self
        }
    }
}
