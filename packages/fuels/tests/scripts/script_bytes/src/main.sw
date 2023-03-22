script;

use std::bytes::Bytes;

enum SomeEnum<T> {
    First: bool,
    Second: T,
}

struct Wrapper<T> {
    inner: T,
    inner_enum: SomeEnum<Bytes>,
}

fn validate_bytes(bytes: Bytes) {
    require(bytes.len() == 3, "bytes len is not 3");
    require(bytes.get(2).unwrap() == 42u8, "expected 3. byte to be 42");
    require(bytes.get(1).unwrap() == 41u8, "expected 2. byte to be 41");
    require(bytes.get(0).unwrap() == 40u8, "expected 1. byte to be 40");
}

fn main(a: u64, wrapper: Wrapper<Vec<Bytes>>) {
    if let SomeEnum::Second(enum_bytes) = wrapper.inner_enum {
        validate_bytes(enum_bytes);
    } else {
        require(false, "enum was not of variant Second");
    }

    require(wrapper.inner.len() == 2, "vec should have two elements");

    let bytes = wrapper.inner.get(0).unwrap();
    validate_bytes(bytes);

    let bytes = wrapper.inner.get(1).unwrap();
    validate_bytes(bytes);
}
