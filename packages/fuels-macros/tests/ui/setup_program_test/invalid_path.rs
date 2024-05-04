use fuels_macros::setup_program_test;

setup_program_test!(Abigen(Contract(
    name = "MyContract",
    project = "some_project"
)));

fn main() {}
