contract;

use std::*;
use core::*;
use std::storage::*;

enum Shaker {
    Cosmopolitan: Recipe,
    Mojito: Cocktail,

}
struct Recipe {
    ice: Chemical,
    sugar: u64,
}

enum Chemical {
    Oxygen: u64,
    Hydrogen: u64,
}

struct Cocktail {
    alcohol: Ethanol,
    glass: u64,
}

enum Ethanol {
   Hydrogen: u64,
   Carbon: u64,
}


abi TestContract {
    fn give_and_return_struct_inside_enum(c: u64) -> Shaker;
}


impl TestContract for Contract {
    fn give_and_return_struct_inside_enum(c: u64) -> Shaker {
        let s = Shaker::Cosmopolitan(
            Recipe{
                ice: Chemical::Oxygen(22),
                sugar: 99
            }
        );
        s
    }
}
