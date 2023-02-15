contract;

configurable {
    STR: str[5] = "hal35",
    ARR: [u8; 3] = [253u8, 254u8, 255u8],
    STR2: str[4] = "fuel",
}



abi TestContract {
fn something() -> (str[5], [u8; 3], str[4]);
}

impl TestContract for Contract {
fn something() -> (str[5], [u8; 3], str[4]) {
    (STR, ARR, STR2)
}
}
