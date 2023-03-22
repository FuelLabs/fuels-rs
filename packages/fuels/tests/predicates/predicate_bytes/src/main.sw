predicate;

use std::bytes::Bytes;

enum SomeEnum<T> {
    First: bool,
    Second: T,
}

struct Wrapper<T> {
    inner: T,
    inner_enum: SomeEnum<Bytes>,
}

fn validate_bytes(bytes: Bytes) -> bool {
    let mut result = bytes.len() == 3;

    result = result && bytes.get(2).unwrap() == 42u8;
    result = result && bytes.get(1).unwrap() == 41u8;
    result && bytes.get(0).unwrap() == 40u8
}

fn main(wrapper: Wrapper<Vec<Bytes>>) -> bool {
    let mut main_result = if let SomeEnum::Second(enum_bytes) = wrapper.inner_enum {
        validate_bytes(enum_bytes)
    } else {
        false
    };

    main_result = main_result && wrapper.inner.len() == 2;

    let bytes = wrapper.inner.get(0).unwrap();
    main_result = main_result && validate_bytes(bytes);

    let bytes = wrapper.inner.get(1).unwrap();
    main_result && validate_bytes(bytes)
}
