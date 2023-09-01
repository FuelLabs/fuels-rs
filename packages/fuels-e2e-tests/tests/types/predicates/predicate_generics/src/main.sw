predicate;

struct GenericStruct<U> {
    value: U,
}

#[allow(dead_code)]
enum GenericEnum<T, V> {
    Generic: GenericStruct<T>,
    AnotherGeneric: V,
}

fn main(
    generic_struct: GenericStruct<u8>,
    generic_enum: GenericEnum<u16, u32>,
) -> bool {
    if let GenericEnum::Generic(other_struct) = generic_enum {
        return other_struct.value == generic_struct.value.as_u16();
    }

    false
}
