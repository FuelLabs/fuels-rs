predicate;

impl PartialEq for StructWithGeneric<u8> {
    fn eq(self, other: Self) -> bool {
        self.field_1 == other.field_1 && self.field_2 == other.field_2
    }
}
impl Eq for StructWithGeneric<u8> {}

impl PartialEq for EnumWithGeneric<bool> {
    fn eq(self, other: Self) -> bool {
        match (self, other) {
            (EnumWithGeneric::VariantOne, EnumWithGeneric::VariantOne) => true,
            (EnumWithGeneric::VariantTwo, EnumWithGeneric::VariantTwo) => true,
            _ => false,
        }
    }
}
impl Eq for EnumWithGeneric<bool> {}

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
    TUPLE: (u8, bool) = (8, true),
    ARRAY: [u32; 3] = [253, 254, 255],
    STRUCT: StructWithGeneric<u8> = StructWithGeneric {
        field_1: 8,
        field_2: 16,
    },
    ENUM: EnumWithGeneric<bool> = EnumWithGeneric::VariantOne(true),
}

fn main(
    switch: bool,
    u_8: u8,
    some_tuple: (u8, bool),
    some_array: [u32; 3],
    some_struct: StructWithGeneric<u8>,
    some_enum: EnumWithGeneric<bool>,
) -> bool {
    switch == BOOL && u_8 == U8 && some_tuple.0 == TUPLE.0 && some_tuple.1 == TUPLE.1 && some_array[0] == ARRAY[0] && some_array[1] == ARRAY[1] && some_array[2] == ARRAY[2] && some_struct == STRUCT && some_enum == ENUM
}
// ANCHOR_END: predicate_configurables
