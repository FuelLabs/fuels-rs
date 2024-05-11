contract;

#[allow(dead_code)]
enum SomeEnum {
    SomeStr: str[4],
    SomeArr: [u64; 4],
}

#[allow(dead_code)]
enum Shaker {
    Cosmopolitan: Recipe,
    Mojito: u32,
}

#[allow(dead_code)]
struct Recipe {
    ice: u8,
    sugar: u16,
}

#[allow(dead_code)]
enum EnumLevel1 {
    Num: u32,
    check: bool,
}

#[allow(dead_code)]
enum EnumLevel2 {
    El1: EnumLevel1,
    Check: bool,
}

#[allow(dead_code)]
enum EnumLevel3 {
    El2: EnumLevel2,
    Num: u8,
}

abi MyContract {
    fn str_inside_enum(my_enum: SomeEnum) -> SomeEnum;
    fn arr_inside_enum(my_enum: SomeEnum) -> SomeEnum;

    fn return_struct_inside_enum(c: u16) -> Shaker;
    fn take_struct_inside_enum(s: Shaker) -> u64;

    fn get_nested_enum() -> EnumLevel3;
    fn check_nested_enum_integrity(e: EnumLevel3) -> bool;
}

impl MyContract for Contract {
    fn str_inside_enum(my_enum: SomeEnum) -> SomeEnum {
        my_enum
    }
    fn arr_inside_enum(my_enum: SomeEnum) -> SomeEnum {
        my_enum
    }

    fn return_struct_inside_enum(c: u16) -> Shaker {
        Shaker::Cosmopolitan(Recipe {
            ice: 22u8,
            sugar: c,
        })
    }
    fn take_struct_inside_enum(_s: Shaker) -> u64 {
        8888
    }

    fn get_nested_enum() -> EnumLevel3 {
        EnumLevel3::El2(EnumLevel2::El1(EnumLevel1::Num(42u32)))
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
}
