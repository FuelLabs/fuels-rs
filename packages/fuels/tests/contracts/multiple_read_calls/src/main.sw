contract;

use std::storage::storage_api::{read, write};

abi MyContract {
    #[storage(write)]
    fn store(input: u64);
    #[storage(read)]
    fn read(input: u64) -> u64;
}

const COUNTER_KEY = 0x0000000000000000000000000000000000000000000000000000000000000000;

impl MyContract for Contract {
    #[storage(write)]
    fn store(input: u64) {
        write(COUNTER_KEY, 0, input);
    }

    #[storage(read)]
    fn read(input: u64) -> u64 {
        let v = read::<u64>(COUNTER_KEY, 0).unwrap_or(0);
        v
    }
}
