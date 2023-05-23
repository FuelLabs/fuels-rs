contract;

use std::block::timestamp;

abi MyContract {
    fn log_timestamp() -> bool;
}

impl MyContract for Contract {
    fn log_timestamp() -> bool {
        log(timestamp());
        true
    }
}
