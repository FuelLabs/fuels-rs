contract;

use std::storage::storage_api::{read, write};

struct EmptyStruct {}

#[allow(dead_code)]
struct CounterConfig {
    dummy: bool,
    initial_value: u64,
}

abi TestContract {
    #[storage(write)]
    fn initialize_counter(config: CounterConfig) -> u64;
    #[storage(read, write)]
    fn increment_counter(amount: u64) -> u64;
    fn get_empty_struct() -> EmptyStruct;
    fn input_empty_struct(es: EmptyStruct) -> bool;
}

const COUNTER_KEY = 0x0000000000000000000000000000000000000000000000000000000000000000;

impl TestContract for Contract {
    #[storage(write)]
    fn initialize_counter(config: CounterConfig) -> u64 {
        let value = config.initial_value;
        write(COUNTER_KEY, 0, value);
        value
    }

    #[storage(read, write)]
    fn increment_counter(amount: u64) -> u64 {
        let value = read::<u64>(COUNTER_KEY, 0).unwrap_or(0) + amount;
        write(COUNTER_KEY, 0, value);
        value
    }

    fn get_empty_struct() -> EmptyStruct {
        EmptyStruct {}
    }

    fn input_empty_struct(es: EmptyStruct) -> bool {
        let EmptyStruct {} = es;

        true
    }
}
