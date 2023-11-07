contract;

abi TestContract {
    fn check_inputs_and_witnesses() -> u64;
}

impl TestContract for Contract {
    // ANCHOR: msg_amount
    fn check_inputs_and_witnesses() -> u64 {
        0
    }
    // ANCHOR_END: msg_amount
}
