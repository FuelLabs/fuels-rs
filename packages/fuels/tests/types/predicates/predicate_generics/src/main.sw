predicate;

struct GenericStruct<U> {
    value: U,
}

enum GenericEnum<T, V> {
    Generic: GenericStruct<T>,
    AnotherGeneric: V,
}

fn main(
    generic_struct: GenericStruct<u8>,
    generic_enum: GenericEnum<u16, u32>,
) -> bool {
    if let GenericEnum::Generic(other_struct) = generic_enum {
        return generic_struct.value == other_struct.value;
    }

    false
}
