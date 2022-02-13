contract;

use std::storage::store;
use std::storage::get;

abi MyContract {
    fn store(gas_: u64, amount_: u64, color_: b256, input: u64);
    fn read(gas_: u64, amount_: u64, color_: b256, input: u64) -> u64;
}

const COUNTER_KEY = 0x0000000000000000000000000000000000000000000000000000000000000000;

impl MyContract for Contract {
    fn store(gas_: u64, amount_: u64, color_: b256, input: u64) {
        store(COUNTER_KEY, input);
    }

    fn read(gas_: u64, amount_: u64, color_: b256, input: u64) -> u64 {
        let v = get(COUNTER_KEY);
        v
    }
}


