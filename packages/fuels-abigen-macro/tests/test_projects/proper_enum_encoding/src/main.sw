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

abi MyContract {
    fn get_bundle_as_constructed_by_sway() -> Bundle;
    fn is_bundle_correct(arg: Bundle) -> bool;
}

impl MyContract for Contract {
    fn get_bundle_as_constructed_by_sway() -> Bundle {
        let arg_1 = EnumThatHasABigAndSmallVariant::Small(12345);
        let arg_2 = 6666;
        let arg_3 = 7777;
        let arg_4 = 8888;
        Bundle {
            arg_1, arg_2, arg_3, arg_4
        }
    }
    fn is_bundle_correct(arg: Bundle) -> bool {
        match arg.arg_1 {
            EnumThatHasABigAndSmallVariant::Small(value) => {
                value == 12345u32 && arg.arg_2 == 6666 && arg.arg_3 == 7777 && arg.arg_4 == 8888
            },
            _ => false, 
        }
    }
}
