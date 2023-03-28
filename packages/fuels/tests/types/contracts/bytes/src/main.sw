contract;

use std::bytes::Bytes;

enum SomeEnum<T> {
    First: bool,
    Second: T,
}

struct Wrapper<T> {
    inner: T,
    inner_enum: SomeEnum<Bytes>,
}

abi MyContract {
    fn accept_bytes(bytes: Bytes);
    fn accept_nested_bytes(wrapper: Wrapper<Vec<Bytes>>);
    fn return_bytes(len: u8) -> Bytes;
}

fn validate_bytes(bytes: Bytes) {
    require(bytes.len() == 3, "bytes len is not 3");
    require(bytes.get(2).unwrap() == 42u8, "expected 3rd byte to be 42");
    require(bytes.get(1).unwrap() == 41u8, "expected 2nd byte to be 41");
    require(bytes.get(0).unwrap() == 40u8, "expected 1st byte to be 40");
}

impl MyContract for Contract {
    fn accept_bytes(bytes: Bytes) {
        validate_bytes(bytes);
    }

    fn accept_nested_bytes(wrapper: Wrapper<Vec<Bytes>>) {
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

    fn return_bytes(len: u8) -> Bytes {
        let mut bytes = Bytes::new();
        let mut i: u8 = 0;
        while i < len {
            bytes.push(i);
            i += 1u8;
        }
        bytes
    }
}
