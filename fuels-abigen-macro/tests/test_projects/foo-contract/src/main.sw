contract;

use foo::FooContract;

impl FooContract for Contract {
    fn foo(gas_: u64, amount_: u64, color_: b256, value: bool) -> bool {
       !value
    }
}
