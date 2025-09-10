script;

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

struct B {
    id: u64,
    val: u64,
}

#[error_type]
enum MyError {
    #[error(m = "some error A")]
    A: (),
    #[error(m = "some complex error B")]
    B: B,
}

#[allow(dead_code)]
enum MatchEnum {
    RequirePrimitive: (),
    RequireString: (),
    RequireCustomGeneric: (),
    RequireWithAdditionalLogs: (),
    RevWLogPrimitive: (),
    RevWLogString: (),
    RevWLogCustomGeneric: (),
    Panic: (),
    PanicError: (),
}

fn main(match_enum: MatchEnum) {
    match match_enum {
        MatchEnum::RequirePrimitive => require(false, 42),
        MatchEnum::RequireString => require(false, __to_str_array("fuel")),
        MatchEnum::RequireCustomGeneric => {
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
        MatchEnum::RequireWithAdditionalLogs => {
            log(42);
            log(__to_str_array("fuel"));
            require(false, 64);
        }
        MatchEnum::RevWLogPrimitive => revert_with_log(42),
        MatchEnum::RevWLogString => revert_with_log(__to_str_array("fuel")),
        MatchEnum::RevWLogCustomGeneric => {
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
        MatchEnum::Panic => panic "some panic message",
        MatchEnum::PanicError => panic MyError::B(B {
            id: 42,
            val: 36,
        }),
    }
}
