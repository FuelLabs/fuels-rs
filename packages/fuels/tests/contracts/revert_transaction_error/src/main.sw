contract;

abi MyContract {
    fn make_transaction_fail(fail: bool) -> u64;
}

impl MyContract for Contract {
    fn make_transaction_fail(fail: bool) -> u64 {
        if fail {
            revert(128);
        }

        42
    }
}
