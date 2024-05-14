contract;

abi MyContract {
    fn b256_as_output() -> b256;
    fn b256_as_input(foo: b256) -> bool;
}

impl MyContract for Contract {
    fn b256_as_output() -> b256 {
        0x0202020202020202020202020202020202020202020202020202020202020202
    }

    fn b256_as_input(foo: b256) -> bool {
        foo == 0x0101010101010101010101010101010101010101010101010101010101010101
    }
}
