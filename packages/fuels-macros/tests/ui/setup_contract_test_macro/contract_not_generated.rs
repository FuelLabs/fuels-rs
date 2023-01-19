use fuels_macros::setup_contract_test;

setup_contract_test!(Deploy(
    name = "some_instance",
    contract = "SomeUnknownContract",
    wallet = "some_wallet"
));

fn main() {}
