contract;

enum EnumThatHasABigAndSmallVariant {
    Small: u32,
    Big: b256,
}

struct AllArgsTogether {
    arg_1: EnumThatHasABigAndSmallVariant,
    arg_2: u64,
    arg_3: u64,
    arg_4: u64,
}

abi MyContract {
    fn test_function(arg_1: EnumThatHasABigAndSmallVariant, arg_2: u64, arg_3: u64, arg_4: u64) -> AllArgsTogether;
}

impl MyContract for Contract {
    fn test_function(arg_1: EnumThatHasABigAndSmallVariant, arg_2: u64, arg_3: u64, arg_4: u64) -> AllArgsTogether {
        AllArgsTogether {
            arg_1, arg_2, arg_3, arg_4
        }
    }
}
