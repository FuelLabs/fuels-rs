contract;

use lib_contract::LibContract;
use std::{asset::mint_to_address, constants::ZERO_B256};

abi ContractCaller {
    fn increment_from_contract(contract_id: ContractId, value: u64) -> u64;
    fn increment_from_contracts(
        contract_id: ContractId,
        contract_id2: ContractId,
        value: u64,
    ) -> u64;
    fn mint_then_increment_from_contract(contract_id: ContractId, amount: u64, address: Address);
    fn require_from_contract(contract_id: ContractId);
    fn re_entrant(contract_id: ContractId, re_enter: bool) -> u64;
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

        contract_instance.increment(value) + contract_instance2.increment(value)
    }

    fn mint_then_increment_from_contract(contract_id: ContractId, amount: u64, address: Address) {
        mint_to_address(address, ZERO_B256, amount);

        let contract_instance = abi(LibContract, contract_id.into());
        let _ = contract_instance.increment(42);
    }

    fn require_from_contract(contract_id: ContractId) {
        let contract_instance = abi(LibContract, contract_id.into());

        contract_instance.require();
    }

    fn re_entrant(contract_id: ContractId, re_enter: bool) -> u64 {
        if !re_enter {
            return 101
        }

        let contract_instance = abi(ContractCaller, contract_id.into());
        let _ = contract_instance.re_entrant(contract_id, false);

        42
    }
}
