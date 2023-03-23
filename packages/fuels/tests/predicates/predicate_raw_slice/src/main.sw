predicate;

enum SomeEnum<T> {
    First: bool,
    Second: T,
}

struct Wrapper<T> {
    inner: T,
    inner_enum: SomeEnum<raw_slice>,
}

fn validate_raw_slice(slice: raw_slice) -> bool {
    let vec: Vec<u64> = Vec::from(slice);

    let mut result = vec.len() == 3;

    result = result && vec.get(2).unwrap() == 42u8;
    result = result && vec.get(1).unwrap() == 41u8;
    result && vec.get(0).unwrap() == 40u8
}

fn main(wrapper: Wrapper<Vec<raw_slice>>) -> bool {
    let mut main_result = if let SomeEnum::Second(enum_raw_slice) = wrapper.inner_enum
    {
        validate_raw_slice(enum_raw_slice)
    } else {
        false
    };

    main_result = main_result && wrapper.inner.len() == 2;

    let slice = wrapper.inner.get(0).unwrap();
    main_result = main_result && validate_raw_slice(slice);

    let slice = wrapper.inner.get(1).unwrap();
    main_result && validate_raw_slice(slice)
}
