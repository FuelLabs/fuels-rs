contract;

use std::logging::log;
use library::ContractLogs;

abi ContractCaller {
    fn logs_from_external_contract(contract_id: ContractId) -> ();
    fn panic_from_external_contract(contract_id: ContractId) -> ();
    fn panic_error_from_external_contract(contract_id: ContractId) -> ();
}

impl ContractCaller for Contract {
    fn logs_from_external_contract(contract_id: ContractId) {
        // Call contract with `contract_id` and make some logs
        let contract_instance = abi(ContractLogs, contract_id.into());
        contract_instance.produce_logs_values();
    }

    fn panic_from_external_contract(contract_id: ContractId) {
        // Call contract with `contract_id` and make some logs
        let contract_instance = abi(ContractLogs, contract_id.into());
        contract_instance.produce_panic();
    }

    fn panic_error_from_external_contract(contract_id: ContractId) {
        // Call contract with `contract_id` and make some logs
        let contract_instance = abi(ContractLogs, contract_id.into());
        contract_instance.produce_panic_with_error();
    }
}
