contract;

abi MyContract {
    fn takes_int_returns_bool(arg: u32) -> bool;
}

impl MyContract for Contract {
    fn takes_int_returns_bool(arg: u32) -> bool {
        arg == 32u32
    }
}
