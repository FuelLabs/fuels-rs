contract;
abi TestContract {
  fn send_message(recipient: b256, msg_len: u8, output: u8, coins: u8) -> bool;
}

use std::constants::ZERO_B256;
impl TestContract for Contract {
    fn send_message(recipient: b256, msg_len: u8, output: u8, coins: u8) -> bool {
        asm(recipient: recipient, msg_len: msg_len, output: output, coins: coins) {
            smo recipient msg_len coins output;
        }
        true
    }
}
