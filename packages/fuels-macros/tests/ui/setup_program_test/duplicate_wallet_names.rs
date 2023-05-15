use fuels_macros::setup_program_test;

setup_program_test!(
    Wallets("wallet1", "wallet1"),
    Abigen(Contract(name = "MyContract", project = "some_file.json"))
);

fn main() {}
