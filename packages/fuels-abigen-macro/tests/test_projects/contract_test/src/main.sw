contract;

use std::*;
use core::*;
use std::storage::*;

struct MyType {
    x: u64,
    y: u64,
}

abi TestContract {
    fn initialize_counter(value: u64) -> u64;
    fn increment_counter(value: u64) -> u64;
    fn get_counter() -> u64;
    fn get(x: u64, y: u64) -> u64;
    fn get_alt(x: MyType) -> u64;
    fn get_single(x: u64) -> u64;
}

const COUNTER_KEY = 0x0000000000000000000000000000000000000000000000000000000000000000;

impl TestContract for Contract {
    fn initialize_counter(value: u64) -> u64 {
        store(COUNTER_KEY, value);
        value
    }
    fn increment_counter(value: u64) -> u64 {
        let new_value = get::<u64>(COUNTER_KEY) + value;
        store(COUNTER_KEY, new_value);
        new_value
    }
    fn get_counter() -> u64 {
        get::<u64>(COUNTER_KEY)
    }

    fn get(x: u64, y: u64) -> u64 {
        x
    }

    fn get_alt(t: MyType) -> u64 {
        t.x
    }

    fn get_single(x: u64) -> u64 {
        x
    }
}
