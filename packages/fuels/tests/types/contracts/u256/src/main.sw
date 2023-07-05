contract;

use std::u256::U256;

#[allow(dead_code)]
enum SomeEnum<T> {
    A: bool,
    B: T,
}

abi MyContract {
    fn u256_sum_and_ret(some_u256: U256) -> U256;
    fn u256_in_enum_input(some_enum: SomeEnum<U256>);
    fn u256_in_enum_output() -> SomeEnum<U256>;
}

impl MyContract for Contract {
    fn u256_sum_and_ret(arg: U256) -> U256 {
        arg + U256::from((3, 4, 5, 6))
    }

    fn u256_in_enum_input(some_enum: SomeEnum<U256>) {
        if let SomeEnum::B(some_u256) = some_enum {
            let expected_u256 = U256::from((2, 3, 4, 5));
            require(some_u256 == expected_u256, "given u256 didn't match the expected u256");
        } else {
            require(false, "enum was not of variant B: u256");
        }
    }

    fn u256_in_enum_output() -> SomeEnum<U256> {
        SomeEnum::B(U256::from((1, 2, 3, 4)))
    }
}
