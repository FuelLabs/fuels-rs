contract;

use std::storage::{get, store};

struct EmptyStruct {}

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
        store(COUNTER_KEY, value);
        value
    }

    #[storage(read, write)]
    fn increment_counter(amount: u64) -> u64 {
        let value = get::<u64>(COUNTER_KEY).unwrap_or(0) + amount;
        store(COUNTER_KEY, value);
        value
    }

    fn get_empty_struct() -> EmptyStruct {
        EmptyStruct {}
    }

    fn input_empty_struct(es: EmptyStruct) -> bool {
        if let EmptyStruct {} = es {
            return true;
        }
        false
    }
}
