use fuels_abigen_macro::abigen;

abigen!(Contract(name = true, abi = "some-abi.json",));

fn main() {}
