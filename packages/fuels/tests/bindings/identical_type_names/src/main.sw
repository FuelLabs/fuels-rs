contract;

dep some_library;
dep original_types;

use original_types::SomeStruct;
use some_library::AWrappingStruct;

abi MyContract {
    fn test_function(a: AWrappingStruct, b: SomeStruct);
}

impl MyContract for Contract {
    fn test_function(a: AWrappingStruct, b: SomeStruct) {
    }
}
