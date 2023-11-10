contract;

abi MyContract {
    fn make_transaction_fail(input: u64) -> u64;
}

impl MyContract for Contract {
    fn make_transaction_fail(input: u64) -> u64 {
        // Work around DCE and revert
        if true {
            revert(input);
        }
        42
    }
}
