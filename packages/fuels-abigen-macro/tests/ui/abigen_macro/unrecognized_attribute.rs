use fuels_abigen_macro::abigen;

abigen!(Contract(
    name = "SomeName",
    abi = "some-abi.json",
    unknown = "something"
));

fn main() {}
