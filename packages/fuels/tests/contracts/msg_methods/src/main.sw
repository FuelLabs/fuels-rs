contract;

use std::auth::msg_sender;

abi FuelTest {
    #[payable]
    fn message_sender() -> Identity;
}

impl FuelTest for Contract {
    #[payable]
    fn message_sender() -> Identity {
        msg_sender().unwrap()
    }
}
