contract;

storage {
    x: u64 = 64,
    y: b256 = 0x0101010101010101010101010101010101010101010101010101010101010101,
}


abi MyContract {
    fn test_function() -> bool;
}

impl MyContract for Contract {
    fn test_function() -> bool {
        true
    }
}
