contract;

use std::storage::storage_api::{read_quads, write_quads};

abi MyContract {
    #[storage(write)]
    fn store(input: u64);
    #[storage(read)]
    fn read() -> u64;
}

const COUNTER_KEY = 0x0000000000000000000000000000000000000000000000000000000000000000;

impl MyContract for Contract {
    #[storage(write)]
    fn store(input: u64) {
        write_quads(COUNTER_KEY, 0, input);
    }

    #[storage(read)]
    fn read() -> u64 {
        read_quads::<u64>(COUNTER_KEY, 0).unwrap_or(0)
    }
}
