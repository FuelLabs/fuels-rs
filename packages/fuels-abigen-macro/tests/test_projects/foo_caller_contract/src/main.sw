contract;

use foo::FooContract;
use std::constants::NATIVE_ASSET_ID;

abi FooCaller {
    fn call_foo_contract(value: bool) -> bool;
}

impl FooCaller for Contract {
    fn call_foo_contract(value: bool) -> bool {
        let foo_contract = abi(FooContract, 0xfe98f602add19c4b2d0c8be2929e4300f9f154eda43457b8b9ea02ef2c7b2d3c);
        let response = foo_contract.foo {
            gas: 10000, coins: 0, asset_id: NATIVE_ASSET_ID
        }
        (value);

        response
    }
}
