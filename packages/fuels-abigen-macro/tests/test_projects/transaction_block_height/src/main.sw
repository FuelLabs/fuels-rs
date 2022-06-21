contract;

abi MyContract {
    fn test_function() -> u64;
}

impl MyContract for Contract {
    fn test_function() -> u64 {
        12345u64
    }
}
