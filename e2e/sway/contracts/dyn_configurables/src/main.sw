contract;

configurable {
    BOOL: bool = true,
    U8: u8 = 8,
    STR: str = "sway",
    STR_2: str = "forc",
    STR_3: str = "fuel",
    LAST_U8: u8 = 16,
}

abi TestContract {
    fn return_configurables() -> (bool, u8, str, str, str, u8);
}

impl TestContract for Contract {
    fn return_configurables() -> (bool, u8, str, str, str, u8) {
        (BOOL, U8, STR, STR_2, STR_3, LAST_U8)
    }
}
