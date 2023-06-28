use fuels_macros::setup_program_test;

setup_program_test!(
    Abigen(Contract(project = "some_project", name = "SomeContract")),
    Deploy(
        name = "some_instance",
        contract = "SomeContract",
        wallet = "some_wallet"
    ),
    UnknownCommand()
);

fn main() {}
