contract;

use foo::FooContract;
use std::constants::NATIVE_ASSET_ID;

abi FooCaller {
    fn call_foo_contract(value: bool) -> bool;
}

impl FooCaller for Contract {
    fn call_foo_contract(value: bool) -> bool {
        let foo_contract = abi(FooContract, 0x5b987da578669aa7f733110c3b1e99678fee8f9dd1302e6562c0a6f35bab4b26);
        let response = foo_contract.foo {
            gas: 10000, coins: 0, asset_id: NATIVE_ASSET_ID
        }
        (value);

        response
    }
}
