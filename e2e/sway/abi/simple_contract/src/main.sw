contract;

abi SimpleContract {
    fn takes_u32_returns_bool(arg: u32) -> bool;
}

impl SimpleContract for Contract {
    fn takes_u32_returns_bool(_arg: u32) -> bool {
        true
    }
}
