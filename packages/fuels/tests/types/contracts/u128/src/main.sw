contract;

use std::u128::U128;

struct MyU128 {
    upper: u64,
    lower: u64
    }

abi MyContract {
    fn u128_as_output() -> U128;
    fn myu128_as_input(some_u128: MyU128) -> bool;
    fn u128_as_input(some_u128: U128) -> bool;
}

impl MyContract for Contract {
    fn u128_as_output() -> U128 {
        U128::from((1, 1))
    }

    fn u128_as_input(some_u128: U128) -> bool {
        log(some_u128);

        log(some_u128.upper);
        log(some_u128.lower);

        let expected_u128 = U128::from((2, 2));
        log(expected_u128.upper);
        log(expected_u128.lower);
        log(expected_u128);

        true
    }

    fn myu128_as_input(some_u128: MyU128) -> bool {
        log(some_u128.upper);
        log(some_u128.lower);

        let expected_u128 = U128::from((2, 2));
        log(expected_u128.upper);
        log(expected_u128.lower);
        log(expected_u128);

        true
    }
}
