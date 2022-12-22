script;

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

enum MatchEnum {
    RequirePrimitive: (),
    RequireString: (),
    RequireCustomGeneric: (),
    RequireWithAdditionalLogs: (),
}

fn main(match_enum: MatchEnum) {
    if let MatchEnum::RequirePrimitive = match_enum {
        require(false, 42);
    } else if let MatchEnum::RequireString = match_enum {
        require(false, "fuel");
    } else if let MatchEnum::RequireCustomGeneric = match_enum {
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
    } else if let MatchEnum::RequireWithAdditionalLogs = match_enum {
        log(42);
        log("fuel");
        require(false, 64);
    }
}
