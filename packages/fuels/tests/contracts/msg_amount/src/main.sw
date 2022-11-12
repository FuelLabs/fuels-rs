contract;

use std::context::msg_amount;

abi FuelTest {
    fn get_msg_amount() -> u64;
}

impl FuelTest for Contract {
    fn get_msg_amount() -> u64 {
        msg_amount()
    }
}
