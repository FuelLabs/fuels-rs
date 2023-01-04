contract;

use foo::FooContract;
use std::constants::ZERO_B256;
use std::token::mint_to_address;

abi FooCaller {
    fn call_foo_contract(target: b256, value: bool) -> bool;
    fn call_foo_contract_then_mint(target: b256, amount: u64, address: Address);
}

impl FooCaller for Contract {
    fn call_foo_contract(target: b256, value: bool) -> bool {
        let foo_contract = abi(FooContract, target);
        let response = foo_contract.foo {
            gas: 10000,
            coins: 0,
            asset_id: ZERO_B256,
        }(value);

        !response
    }

    fn call_foo_contract_then_mint(target: b256, amount: u64, address: Address) {
        let foo_contract = abi(FooContract, target);
        let response = foo_contract.foo {
            gas: 1000000,
            coins: 0,
            asset_id: ZERO_B256,
        }(true);

        mint_to_address(amount, address);
    }
}
