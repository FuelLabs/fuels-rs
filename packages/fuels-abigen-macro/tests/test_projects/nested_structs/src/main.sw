contract;

pub struct SomeStruct {
    par1: u32,
}

pub struct AllStruct {
    some_struct: SomeStruct,
}

abi MyContract {
    fn get_struct() -> AllStruct;
    fn check_struct_integrity(arg: AllStruct) -> bool;
}

impl MyContract for Contract {
    fn get_struct() -> AllStruct {
        AllStruct {
            some_struct: SomeStruct {
                par1: 12345u32,
            },
        }
    }
    fn check_struct_integrity(arg: AllStruct) -> bool {
        arg.some_struct.par1 == 12345u32
    }
}
