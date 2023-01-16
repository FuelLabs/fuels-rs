contract;

abi MyContract {
    fn takes_ints_returns_bool(only_argument: u32) -> bool;
}

impl MyContract for Contract {
    fn takes_ints_returns_bool(only_argument: u32) -> bool {
        true
    }
}
