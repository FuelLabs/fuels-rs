contract;

enum StandardEnum {
    One: b256,
    Two: u32,
    Three: bool,
}

enum UnitEnum {
    One: (),
    Two: (),
    Three: (),
}

abi EnumTesting {
    fn get_standard_enum() -> StandardEnum;
    fn check_standard_enum_integrity(arg: StandardEnum) -> bool;

    fn get_unit_enum() -> UnitEnum;
    fn check_unit_enum_integrity(arg: UnitEnum) -> bool;
}

impl EnumTesting for Contract {
    fn get_standard_enum() -> StandardEnum {
        StandardEnum::Two(12345)
    }
    fn check_standard_enum_integrity(arg: StandardEnum) -> bool {
        match arg {
            StandardEnum::Two(value) => {
                value == 12345u32
            },
            _ => {
                false
            }
        }
    }

    fn get_unit_enum() -> UnitEnum {
        UnitEnum::Two
    }
    fn check_unit_enum_integrity(arg: UnitEnum) -> bool {
        match arg {
            UnitEnum::Two => {
                true
            },
            _ => {
                false
            }
        }
    }
}
