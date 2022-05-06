contract;

use foo::FooContract;
use std::constants::NATIVE_ASSET_ID;
use std::contract_id::ContractId;

abi FooCaller {
    fn call_foo_contract(target: b256, value: bool) -> bool;
}

impl FooCaller for Contract {
    fn call_foo_contract(target: b256, value: bool) -> bool {
        let foo_contract = abi(FooContract, target);
        let response = foo_contract.foo {
            gas: 10000, coins: 0, asset_id: NATIVE_ASSET_ID
        }
        (value);

        response
    }
}
