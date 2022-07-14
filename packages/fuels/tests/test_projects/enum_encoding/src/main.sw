contract;

enum EnumThatHasABigAndSmallVariant {
    Big: b256,
    Small: u32,
}

struct BigBundle {
    arg_1: EnumThatHasABigAndSmallVariant,
    arg_2: u64,
    arg_3: u64,
    arg_4: u64,
}

enum UnitEnum {
    var1: (),
    var2: (),
}

struct UnitBundle {
    arg_1: UnitEnum,
    arg_2: u64,
}

abi EnumTesting {
    fn get_big_bundle() -> BigBundle;
    fn check_big_bundle_integrity(arg: BigBundle) -> bool;

    fn get_unit_bundle() -> UnitBundle;
    fn check_unit_bundle_integrity(arg: UnitBundle) -> bool;
}

impl EnumTesting for Contract {
    fn get_big_bundle() -> BigBundle {
        let arg_1 = EnumThatHasABigAndSmallVariant::Small(12345);
        let arg_2 = 6666;
        let arg_3 = 7777;
        let arg_4 = 8888;
        BigBundle {
            arg_1, arg_2, arg_3, arg_4
        }
    }
    fn check_big_bundle_integrity(arg: BigBundle) -> bool {
        let arg_1_is_correct = match arg.arg_1 {
            EnumThatHasABigAndSmallVariant::Small(value) => {
                value == 12345u32
            },
            _ => false, 
        };

        arg_1_is_correct && arg.arg_2 == 6666 && arg.arg_3 == 7777 && arg.arg_4 == 8888
    }

    fn get_unit_bundle() -> UnitBundle {
        UnitBundle {
            arg_1: UnitEnum::var2(),
            arg_2: 18_446_744_073_709_551_615u64,
        }
    }
    fn check_unit_bundle_integrity(arg: UnitBundle) -> bool {
        let arg_1_is_correct = match arg.arg_1 {
            UnitEnum::var2(_) => true, _ => false, 
        };

        arg_1_is_correct && arg.arg_2 == 18_446_744_073_709_551_615u64
    }
}
