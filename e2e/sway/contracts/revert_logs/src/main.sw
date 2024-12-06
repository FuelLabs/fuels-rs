contract;

use std::logging::log;

#[allow(dead_code)]
enum EnumWithGeneric<D> {
    VariantOne: D,
    VariantTwo: (),
}

#[allow(dead_code)]
struct StructWithNestedGeneric<D> {
    field_1: D,
    field_2: u64,
}

#[allow(dead_code)]
struct StructDeeplyNestedGeneric<D> {
    field_1: D,
    field_2: u64,
}

abi TestContract {
    fn require_primitive();
    fn require_string();
    fn require_custom_generic();
    fn require_with_additional_logs();

    fn rev_w_log_primitive();
    fn rev_w_log_string();
    fn rev_w_log_custom_generic();
}

impl TestContract for Contract {
    fn require_primitive() {
        require(false, 42);
    }

    fn require_string() {
        require(false, __to_str_array("fuel"));
    }

    fn require_custom_generic() {
        let l: [u8; 3] = [1u8, 2u8, 3u8];

        let test_enum = EnumWithGeneric::VariantOne(l);
        let test_struct_nested = StructWithNestedGeneric {
            field_1: test_enum,
            field_2: 64,
        };
        let test_deeply_nested_generic = StructDeeplyNestedGeneric {
            field_1: test_struct_nested,
            field_2: 64,
        };

        require(false, test_deeply_nested_generic);
    }

    fn require_with_additional_logs() {
        log(42);
        log(__to_str_array("fuel"));
        require(false, 64);
    }

    fn rev_w_log_primitive() {
        revert_with_log(42);
    }

    fn rev_w_log_string() {
        revert_with_log(__to_str_array("fuel"));
    }

    fn rev_w_log_custom_generic() {
        let l: [u8; 3] = [1u8, 2u8, 3u8];

        let test_enum = EnumWithGeneric::VariantOne(l);
        let test_struct_nested = StructWithNestedGeneric {
            field_1: test_enum,
            field_2: 64,
        };
        let test_deeply_nested_generic = StructDeeplyNestedGeneric {
            field_1: test_struct_nested,
            field_2: 64,
        };

        revert_with_log(test_deeply_nested_generic);
    }
}
