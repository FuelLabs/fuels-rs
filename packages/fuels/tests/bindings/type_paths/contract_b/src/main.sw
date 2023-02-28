contract;

use shared_lib::VeryCommonNameStruct;

abi MyContract {
    fn test_function(arg: VeryCommonNameStruct);
}

impl MyContract for Contract {
    fn test_function(arg: VeryCommonNameStruct) {}
}
