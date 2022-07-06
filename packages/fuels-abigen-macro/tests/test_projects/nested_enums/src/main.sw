contract;

use std::{address::Address, constants::ZERO_B256, identity::Identity, option::Option};

enum EnumLevel1 {
    Num: u32,
    check: bool,
}

enum EnumLevel2 {
    El1: EnumLevel1,
    Check: bool,
}

enum EnumLevel3 {
    El2: EnumLevel2,
    Num: u8,
}

abi MyContract {
    fn get_nested_enum() -> EnumLevel3;
    fn check_nested_enum_integrity(e: EnumLevel3) -> bool;
    fn get_some_address() -> Option<Identity>;
    fn get_none() -> Option<Identity>;
}

impl MyContract for Contract {
    fn get_nested_enum() -> EnumLevel3 {
        EnumLevel3::El2(EnumLevel2::El1(EnumLevel1::Num(42)))
    }

    fn check_nested_enum_integrity(e: EnumLevel3) -> bool {
        let arg_is_correct = match e {
            EnumLevel3::El2(EnumLevel2::El1(EnumLevel1::Num(value))) => {
                value == 42u32
            },
            _ => false, 
        };

        arg_is_correct
    }

    fn get_some_address() -> Option<Identity> {
        Option::Some(Identity::Address(~Address::from(ZERO_B256)))
    }

    fn get_none() -> Option<Identity> {
        Option::None
    }
}
