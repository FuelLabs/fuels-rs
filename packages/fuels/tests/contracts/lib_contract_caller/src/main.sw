contract;

use foo::LibContract;

abi ContractCaller {
    fn increment_from_contract(contract_id: ContractId, value: u64) -> u64;
    fn require_from_contract(contract_id: ContractId);
}

impl ContractCaller for Contract {
    fn increment_from_contract(contract_id: ContractId, value: u64) -> u64 {
        let contract_instance = abi(LibContract, contract_id.into());

        contract_instance.increment(value)
    }

    fn require_from_contract(contract_id: ContractId) {
        let contract_instance = abi(LibContract, contract_id.into());

        contract_instance.require();
    }
}
