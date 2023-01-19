use fuels_macros::abigen;

abigen!(Contract(
    abi = "some-abi.json",
    abi = "some-abi2.json",
    name = "SomeName",
    abi = "some-abi3.json",
));

fn main() {}
