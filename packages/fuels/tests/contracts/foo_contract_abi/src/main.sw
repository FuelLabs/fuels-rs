library foo_contract_abi;

abi FooContract {
    fn foo(value: bool) -> bool;
    fn mint_to_address(mint_amount: u64, address: Address);
}
