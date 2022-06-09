contract;

enum OnlyUnitsInEnum {
    a: (),
    b: (),
}

struct JointRet {
    arg1: OnlyUnitsInEnum,
    arg2: b256,
}

abi MyContract {
    fn test_function(arg: b256) -> JointRet;
}

impl MyContract for Contract {
    fn test_function(arg: b256) -> JointRet {
        JointRet {
            arg1: OnlyUnitsInEnum::b(),
            arg2: arg,
        }
    }
}
