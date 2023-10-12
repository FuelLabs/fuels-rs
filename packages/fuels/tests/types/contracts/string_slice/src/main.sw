contract;

abi RawSliceContract {
    fn return_str() -> str;
}

impl RawSliceContract for Contract {
    fn return_str() -> str {
        "contract-return"
    }
}
