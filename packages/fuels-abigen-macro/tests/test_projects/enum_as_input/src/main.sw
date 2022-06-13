contract;

enum StandardEnum {
    one: b256,
    two: u32,
    three: bool,
}

enum UnitEnum {
    one: (),
    two: (),
    three: (),
}

abi EnumTesting {
    fn get_standard_enum() -> StandardEnum;
    fn check_standard_enum_integrity(arg: StandardEnum) -> bool;

    fn get_unit_enum() -> UnitEnum;
    fn check_unit_enum_integrity(arg: UnitEnum) -> bool;
}

impl EnumTesting for Contract {
    fn get_standard_enum() -> StandardEnum {
        StandardEnum::two(12345)
    }
    fn check_standard_enum_integrity(arg: StandardEnum) -> bool {
        match arg {
            StandardEnum::two(value) => {
                value == 12345u32
            },
            _ => {
                false
            }
        }
    }

    fn get_unit_enum() -> UnitEnum {
        UnitEnum::two()
    }
    fn check_unit_enum_integrity(arg: UnitEnum) -> bool {
        match arg {
            UnitEnum::two(_) => {
                true
            },
            _ => {
                false
            }
        }
    }
}
