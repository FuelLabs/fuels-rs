contract;

abi MyContract {
    fn returns_tuple(input: (u64, u64)) -> (u64, u64);
}

impl MyContract for Contract {
    fn returns_tuple(input: (u64, u64)) -> (u64, u64) {
        input
    }
}
