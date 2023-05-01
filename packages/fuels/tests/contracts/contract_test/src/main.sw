contract;

use std::storage::storage_api::{read, write};
use std::context::msg_amount;

struct MyType {
    x: u64,
    y: u64,
}

struct Person {
    name: str[4],
}

enum State {
    A: (),
    B: (),
    C: (),
}

abi TestContract {
    #[storage(write)]
    fn initialize_counter(value: u64) -> u64;
    #[storage(read, write)]
    fn increment_counter(value: u64) -> u64;
    #[storage(read)]
    fn get_counter() -> u64;
    fn get(x: u64, y: u64) -> u64;
    fn get_alt(x: MyType) -> MyType;
    fn get_single(x: u64) -> u64;
    fn array_of_structs(p: [Person; 2]) -> [Person; 2];
    fn array_of_enums(p: [State; 2]) -> [State; 2];
    fn get_array(p: [u64; 2]) -> [u64; 2];
    #[payable]
    fn get_msg_amount() -> u64;
    fn new() -> u64;
}

const COUNTER_KEY = 0x0000000000000000000000000000000000000000000000000000000000000000;

impl TestContract for Contract {
    // ANCHOR: msg_amount
    #[payable]
    fn get_msg_amount() -> u64 {
        msg_amount()
    }
    // ANCHOR_END: msg_amount
    #[storage(write)]
    fn initialize_counter(value: u64) -> u64 {
        write(COUNTER_KEY, 0, value);
        value
    }

    #[storage(read, write)]
    fn increment_counter(value: u64) -> u64 {
        let new_value = read::<u64>(COUNTER_KEY, 0).unwrap_or(0) + value;
        write(COUNTER_KEY, 0, new_value);
        new_value
    }

    #[storage(read)]
    fn get_counter() -> u64 {
        read::<u64>(COUNTER_KEY, 0).unwrap_or(0)
    }

    fn get(x: u64, y: u64) -> u64 {
        x
    }

    fn get_alt(t: MyType) -> MyType {
        t
    }

    fn get_single(x: u64) -> u64 {
        x
    }

    fn array_of_structs(p: [Person; 2]) -> [Person; 2] {
        p
    }

    fn array_of_enums(p: [State; 2]) -> [State; 2] {
        p
    }

    fn get_array(p: [u64; 2]) -> [u64; 2] {
        p
    }

    fn new() -> u64 {
        12345u64
    }
}
