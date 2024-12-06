contract;

configurable {
    BOOL: bool = true,
    U8: u8 = 8,
    STRING_SLICE: str = "Hello, Sway",
    LAST_U8: u8 = 16,
}

abi TestContract {
    fn return_configurables() -> (bool, u8, str, u8);
}

impl TestContract for Contract {
    fn return_configurables() -> (bool, u8, str, u8) {
        (BOOL, U8, STRING_SLICE, LAST_U8)
    }
}
