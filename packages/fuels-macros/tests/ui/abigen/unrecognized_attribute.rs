use fuels_macros::abigen;

abigen!(Contract(
    name = "SomeName",
    abi = "some-abi.json",
    unknown = "something"
));

fn main() {}
