contract;

use std::storage::store;
use std::storage::get;

abi MyContract {
    fn store(input: u64);
    fn read(input: u64) -> u64;
}

const COUNTER_KEY = 0x0000000000000000000000000000000000000000000000000000000000000000;

impl MyContract for Contract {
    fn store(input: u64) {
        store(COUNTER_KEY, input);
    }

    fn read(input: u64) -> u64 {
        let v = get::<u64>(COUNTER_KEY);
        v
    }
}
