contract;

enum EnumThatHasABigAndSmallVariant {
    Big: b256,
    Small: u32,
}

struct Bundle {
    arg_1: EnumThatHasABigAndSmallVariant,
    arg_2: u64,
    arg_3: u64,
    arg_4: u64,
}

abi EnumTesting {
    fn get_bundle() -> Bundle;
    fn check_bundle_integrity(arg: Bundle) -> bool;
}

impl EnumTesting for Contract {
    fn get_bundle() -> Bundle {
        let arg_1 = EnumThatHasABigAndSmallVariant::Small(12345);
        let arg_2 = 6666;
        let arg_3 = 7777;
        let arg_4 = 8888;
        Bundle {
            arg_1, arg_2, arg_3, arg_4
        }
    }
    fn check_bundle_integrity(arg: Bundle) -> bool {
        let arg_1_is_correct = match arg.arg_1 {
            EnumThatHasABigAndSmallVariant::Small(value) => {
                value == 12345u32
            },
            _ => false, 
        };

        arg_1_is_correct && arg.arg_2 == 6666 && arg.arg_3 == 7777 && arg.arg_4 == 8888
    }
}
