contract;

abi RawSliceContract {
    fn handles_str(input: str) -> str;
}

impl RawSliceContract for Contract {
    fn handles_str(input: str) -> str {
        assert_eq(input, "contract-input");

        "contract-return"
    }
}
