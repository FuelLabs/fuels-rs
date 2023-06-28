use fuels_macros::abigen;

abigen!(SomeInvalidProgramType(
    name = "SomeName",
    abi = "some-abi.json"
));

fn main() {}
