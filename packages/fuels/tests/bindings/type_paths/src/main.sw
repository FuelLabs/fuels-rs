contract;

mod contract_a_types;
mod another_lib;

use contract_a_types::AWrapper;
use another_lib::VeryCommonNameStruct;

abi MyContract {
    fn test_function(arg: AWrapper) -> VeryCommonNameStruct;
}

impl MyContract for Contract {
    fn test_function(arg: AWrapper) -> VeryCommonNameStruct {
        VeryCommonNameStruct { field_a: 10u32 }
    }
}
