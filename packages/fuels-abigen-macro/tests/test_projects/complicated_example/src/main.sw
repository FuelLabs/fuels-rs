contract;

abi MyContract {
    fn test_function(arg1: AnotherEnum, arg2: SomeStruct, arg3: SomeEnum) -> AllStruct;
    fn def_ok() -> AllStruct;
}

pub enum SomeEnum {
    V1: u32,
    V2: b256,
}

pub enum AnotherEnum {
    en1: SomeStruct,
    en2: u32,
}

pub struct SomeStruct {
    par1: SomeEnum,
    par2: u32,
}

pub struct AllStruct {
    par1: AnotherEnum,
    par2: SomeStruct,
    par3: SomeEnum,
}

impl MyContract for Contract {
    fn test_function(arg1: AnotherEnum, arg2: SomeStruct, arg3: SomeEnum) -> AllStruct {
        AllStruct {
            par1: arg1,
            par2: arg2,
            par3: arg3,
        }
    }
    fn def_ok() -> AllStruct {
        let arg1 = AnotherEnum::en1(SomeStruct {
            par1: SomeEnum::V1(1), par2: 2, 
        });
        let arg2 = SomeStruct {
            par1: SomeEnum::V1(3),
            par2: 4,
        };
        let arg3 = SomeEnum::V1(5);
        AllStruct {
            par1: arg1,
            par2: arg2,
            par3: arg3,
        }
    }
}
