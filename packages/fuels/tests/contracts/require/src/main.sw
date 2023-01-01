contract;

use std::logging::log;

enum EnumWithGeneric<D> {
    VariantOne: D,
    VariantTwo: (),
}

struct StructWithNestedGeneric<D> {
    field_1: D,
    field_2: u64,
}

struct StructDeeplyNestedGeneric<D> {
    field_1: D,
    field_2: u64,
}

abi TestContract {
    fn require_primitive();
    fn require_string();
    fn require_custom_generic();
    fn require_with_additional_logs();
}

impl TestContract for Contract {
    fn require_primitive() {
        require(false, 42);
    }

    fn require_string() {
        require(false, "fuel");
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
        log("fuel");
        require(false, 64);
    }
}
