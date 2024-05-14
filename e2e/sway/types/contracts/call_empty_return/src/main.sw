contract;

use std::storage::storage_api::write;

abi TestContract {
    #[storage(write)]
    fn store_value(val: u64);
}

const COUNTER_KEY = 0x0000000000000000000000000000000000000000000000000000000000000000;

impl TestContract for Contract {
    #[storage(write)]
    fn store_value(val: u64) {
        write(COUNTER_KEY, 0, val);
    }
}
