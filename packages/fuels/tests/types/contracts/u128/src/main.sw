contract;

use std::u128::U128;

enum SomeEnum<T> {
    A: bool,
    B: T,
}

abi MyContract {
    fn u128_as_output() -> U128;
    fn u128_as_input(some_u128: U128);
    fn u128_in_enum_input(some_enum: SomeEnum<U128>);
    fn u128_in_enum_output() -> SomeEnum<U128>;
}

impl MyContract for Contract {
    fn u128_as_output() -> U128 {
        U128::from((1, 1))
    }

    fn u128_as_input(some_u128: U128) {
        let expected_u128 = U128::from((2, 2));
        require(some_u128 == expected_u128, "given u128 didn't match the expected u128");
    }

    fn u128_in_enum_input(some_enum: SomeEnum<U128>) {
        if let SomeEnum::B(some_u128) = some_enum {
            let expected_u128 = U128::from((3, 3));
            require(some_u128 == expected_u128, "given u128 didn't match the expected u128");
        }
        require(false, "enum was not of variant B: u128");
    }

    fn u128_in_enum_output() -> SomeEnum<U128> {
        SomeEnum::B(U128::from((4, 4)))
    }
}
