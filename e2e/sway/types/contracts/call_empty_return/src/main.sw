contract;

use std::storage::storage_api::write_quads;

abi TestContract {
    #[storage(write)]
    fn store_value(val: u64);
}

const COUNTER_KEY = 0x0000000000000000000000000000000000000000000000000000000000000000;

impl TestContract for Contract {
    #[storage(write)]
    fn store_value(val: u64) {
        write_quads(COUNTER_KEY, 0, val);
    }
}
