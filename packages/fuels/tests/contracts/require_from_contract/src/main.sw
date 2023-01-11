contract;

use std::logging::log;
use library::ContractLogs;

abi ContractCaller {
    fn require_from_external_contract(contract_id: ContractId) -> ();
}

impl ContractCaller for Contract {
    fn require_from_external_contract(contract_id: ContractId) {
        let contract_instance = abi(ContractLogs, contract_id.into());
        contract_instance.require_from_contract();
    }
}
