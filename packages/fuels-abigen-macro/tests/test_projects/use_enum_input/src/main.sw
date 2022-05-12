contract;

use std::*;
use core::*;
use std::storage::*;

enum Shaker {
    Cosmopolitan:u8,
    Mojito:u8,
}

abi TestContract {
    fn use_enum_as_input(s: Shaker) -> u64;
}


impl TestContract for Contract {
    fn use_enum_as_input(s: Shaker) -> u64{
        9876
    }
}
