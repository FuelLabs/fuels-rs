contract;

use foo::FooContract;
use std::token::mint_to_address;

impl FooContract for Contract {
    fn foo(value: bool) -> bool {
        !value
    }

    fn mint_to_address(mint_amount: u64, address: Address) {
        mint_to_address(mint_amount, address);
    }
}
