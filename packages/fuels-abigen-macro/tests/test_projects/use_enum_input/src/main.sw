contract;

use std::*;
use core::*;
use std::storage::*;

enum Shaker {
    Cosmopolitan:u8,
    Mojito:u8,
}

enum BimBamBoum {
    Bim: (),
    Bam: (),
    Boum: (),
}

abi TestContract {
    fn use_enum_as_input(s: Shaker) -> u64;
    fn use_unit_type_enum(w: BimBamBoum) -> BimBamBoum;
}


impl TestContract for Contract {
    fn use_enum_as_input(s: Shaker) -> u64{
        9876
    }
    fn use_unit_type_enum(w: BimBamBoum) -> BimBamBoum{
        BimBamBoum::Boum
    }
}
