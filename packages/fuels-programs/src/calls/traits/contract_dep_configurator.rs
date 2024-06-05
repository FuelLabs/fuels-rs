use fuels_core::types::bech32::Bech32ContractId;

use crate::calls::{
    utils::{new_variable_outputs, sealed},
    ContractCall, ScriptCall,
};

pub trait ContractDependencyConfigurator: sealed::Sealed {
    fn append_contract(&mut self, contract_id: Bech32ContractId);
    fn append_variable_outputs(&mut self, num: u64);
    fn with_external_contracts(self, external_contracts: Vec<Bech32ContractId>) -> Self;
}

impl ContractDependencyConfigurator for ContractCall {
    fn append_contract(&mut self, contract_id: Bech32ContractId) {
        self.external_contracts.push(contract_id)
    }

    fn append_variable_outputs(&mut self, num: u64) {
        self.variable_outputs
            .extend(new_variable_outputs(num as usize));
    }

    fn with_external_contracts(self, external_contracts: Vec<Bech32ContractId>) -> Self {
        ContractCall {
            external_contracts,
            ..self
        }
    }
}

impl ContractDependencyConfigurator for ScriptCall {
    fn append_contract(&mut self, contract_id: Bech32ContractId) {
        self.external_contracts.push(contract_id)
    }

    fn append_variable_outputs(&mut self, num: u64) {
        self.variable_outputs
            .extend(new_variable_outputs(num as usize));
    }

    fn with_external_contracts(self, external_contracts: Vec<Bech32ContractId>) -> Self {
        ScriptCall {
            external_contracts,
            ..self
        }
    }
}
