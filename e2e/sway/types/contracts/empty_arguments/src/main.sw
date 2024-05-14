contract;

abi TestContract {
    fn method_with_empty_argument() -> u64;
}

impl TestContract for Contract {
    fn method_with_empty_argument() -> u64 {
        63
    }
}
