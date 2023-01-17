use fuels_abigen_macro::setup_contract_test;

setup_contract_test!(
    Wallets("wallet1"),
    Wallets("wallet2"),
    Abigen(name = "MyContract", abi = "some_file.json")
);

fn main() {}
