use fuels_abigen_macro::abigen;

abigen!(SomeInvalidProgramType(
    name = "SomeName",
    abi = "some-abi.json"
));

fn main() {}
