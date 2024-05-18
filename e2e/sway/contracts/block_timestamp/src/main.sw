contract;

use std::block::timestamp;

abi MyContract {
    fn return_timestamp() -> u64;
}

impl MyContract for Contract {
    fn return_timestamp() -> u64 {
        timestamp()
    }
}
