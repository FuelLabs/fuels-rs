contract;

use std::bytes::Bytes;
use std::string::String;

#[allow(dead_code)]
struct StructGenerics<T, K, U> {
    one: T,
    two: K,
    three: U,
}

#[allow(dead_code)]
enum EnumGeneric<H, I> {
    One: H,
    Two: I,
}

abi HeapTypesContract {
    fn nested_heap_types() -> EnumGeneric<Vec<StructGenerics<Bytes, String, raw_slice>>, String>;
}

impl HeapTypesContract for Contract {
    fn nested_heap_types() -> EnumGeneric<Vec<StructGenerics<Bytes, String, raw_slice>>, String> {
        let mut some_vec = Vec::new();
        some_vec.push(2u8);
        some_vec.push(4u8);
        some_vec.push(8u8);

        let struct_generics = StructGenerics {
            one: Bytes::from(some_vec),
            two: String::from_ascii_str("fuel"),
            three: some_vec.as_raw_slice(),
        };

        let mut enum_vec = Vec::new();
        enum_vec.push(struct_generics);
        enum_vec.push(struct_generics);

        EnumGeneric::One(enum_vec)
    }
}
