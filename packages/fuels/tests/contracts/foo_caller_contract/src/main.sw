contract;

use foo::FooContract;
use std::constants::ZERO_B256;
use std::contract_id::ContractId;

abi FooCaller {
    fn call_foo_contract(target: b256, value: bool) -> bool;
}

impl FooCaller for Contract {
    fn call_foo_contract(target: b256, value: bool) -> bool {
        let foo_contract = abi(FooContract, target);
        let response = foo_contract.foo {
            gas: 10000, coins: 0, asset_id: ZERO_B256
        }
        (value);

        !response
    }
}
