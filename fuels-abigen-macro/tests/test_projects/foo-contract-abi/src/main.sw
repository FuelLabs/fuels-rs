library foo_contract_abi;

abi FooContract {
    fn foo(gas_: u64, amount_: u64, color_: b256, value: bool) -> bool;
}
