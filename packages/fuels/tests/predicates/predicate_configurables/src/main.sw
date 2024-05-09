predicate;

impl Eq for StructWithGeneric<u8> {
    fn eq(self, other: Self) -> bool {
        self.field_1 == other.field_1 && self.field_2 == other.field_2
    }
}

impl Eq for EnumWithGeneric<bool> {
    fn eq(self, other: Self) -> bool {
        match (self, other) {
            (EnumWithGeneric::VariantOne, EnumWithGeneric::VariantOne) => true,
            (EnumWithGeneric::VariantTwo, EnumWithGeneric::VariantTwo) => true,
            _ => false,
        }
    }
}

// ANCHOR: predicate_configurables
#[allow(dead_code)]
enum EnumWithGeneric<D> {
    VariantOne: D,
    VariantTwo: (),
}

struct StructWithGeneric<D> {
    field_1: D,
    field_2: u64,
}

configurable {
    BOOL: bool = true,
    U8: u8 = 8,
    STRUCT: StructWithGeneric<u8> = StructWithGeneric {
        field_1: 8,
        field_2: 16,
    },
    ENUM: EnumWithGeneric<bool> = EnumWithGeneric::VariantOne(true),
}

fn main(
    switch: bool,
    u_8: u8,
    some_struct: StructWithGeneric<u8>,
    some_enum: EnumWithGeneric<bool>,
) -> bool {
    switch == BOOL && u_8 == U8 && some_struct == STRUCT && some_enum == ENUM
}
// ANCHOR_END: predicate_configurables
