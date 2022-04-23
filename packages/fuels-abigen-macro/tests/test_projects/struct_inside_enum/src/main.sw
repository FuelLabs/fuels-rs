contract;

use std::*;
use core::*;
use std::storage::*;

enum Shaker {
    Cosmopolitan: Recipe,
    Mojito: u32,

}
struct Recipe {
    ice: u8,
    sugar: u16,
}

abi TestContract {
    fn return_struct_inside_enum(c: u64) -> Shaker;
}


impl TestContract for Contract {
    fn return_struct_inside_enum(c: u64) -> Shaker {
        let s = Shaker::Cosmopolitan(
            Recipe{
                ice: 22,
                sugar: 99
            }
        );
        s
    }
}
