contract;

use lib_contract::LibContract;
use std::token::mint_to_address;

abi ContractCaller {
    fn increment_from_contract(contract_id: ContractId, value: u64) -> u64;
    fn increment_from_contracts(contract_id: ContractId, contract_id2: ContractId, value: u64) -> u64;
    fn increment_from_contract_then_mint(contract_id: ContractId, amount: u64, address: Address);
    fn require_from_contract(contract_id: ContractId);
}

impl ContractCaller for Contract {
    fn increment_from_contract(contract_id: ContractId, value: u64) -> u64 {
        let contract_instance = abi(LibContract, contract_id.into());

        contract_instance.increment(value)
    }

    fn increment_from_contracts(
        contract_id: ContractId,
        contract_id2: ContractId,
        value: u64,
    ) -> u64 {
        let contract_instance = abi(LibContract, contract_id.into());
        let contract_instance2 = abi(LibContract, contract_id2.into());

        contract_instance.increment(value) + contract_instance.increment(value)
    }

    fn increment_from_contract_then_mint(contract_id: ContractId, amount: u64, address: Address) {
        let contract_instance = abi(LibContract, contract_id.into());
        let response = contract_instance.increment(42);

        mint_to_address(amount, address);
    }

    fn require_from_contract(contract_id: ContractId) {
        let contract_instance = abi(LibContract, contract_id.into());

        contract_instance.require();
    }
}
