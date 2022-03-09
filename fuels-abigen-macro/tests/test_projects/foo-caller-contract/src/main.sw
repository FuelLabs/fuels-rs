contract;

use foo::FooContract;
use std::constants::ETH_ID;

abi FooCaller {
    fn call_foo_contract(gas_: u64, amount_: u64, color_: b256, value: bool) -> bool;
}

impl FooCaller for Contract {
    fn call_foo_contract(gas_: u64, amount_: u64, color_: b256, value: bool) -> bool {
        let foo_contract = abi(FooContract, 0x7b4837e641d659a0662183f0fdfeca3fb4fa1248d62e4721ff28808bf11bd8c7);
        let response = foo_contract.foo(10000, 0, ETH_ID, value);
        response
    }
}
