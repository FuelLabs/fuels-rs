contract;

pub struct SmallStruct {
    foo: u32,
}

pub struct LargeStruct {
    foo: u8,
    bar: u8,
}

abi TestContract {
    fn get_id() -> b256;
    fn get_small_string() -> str[8];
    fn get_large_string() -> str[9];
    fn get_large_struct() -> LargeStruct;
    fn get_small_struct() -> SmallStruct;
    fn get_large_array() -> [u32; 2];
    fn get_contract_id() -> ContractId;
}

impl TestContract for Contract {
    fn get_id() -> b256 {
        0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
    }

    fn get_small_string() -> str[8] {
        let my_string: str[8] = __to_str_array("gggggggg");
        my_string
    }

    fn get_large_string() -> str[9] {
        let my_string: str[9] = __to_str_array("ggggggggg");
        my_string
    }

    fn get_small_struct() -> SmallStruct {
        SmallStruct { foo: 100u32 }
    }

    fn get_large_struct() -> LargeStruct {
        LargeStruct {
            foo: 12u8,
            bar: 42u8,
        }
    }

    fn get_large_array() -> [u32; 2] {
        let x: [u32; 2] = [1u32, 2u32];
        x
    }

    fn get_contract_id() -> ContractId {
        let id = 0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF;
        ContractId::from(id)
    }
}
