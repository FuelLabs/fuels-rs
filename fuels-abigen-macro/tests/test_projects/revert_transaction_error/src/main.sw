contract;

abi MyContract {
    fn make_transaction_fail(gas_: u64, amount_: u64, color_: b256, input: u64) -> u64;
}

const COUNTER_KEY = 0x0000000000000000000000000000000000000000000000000000000000000000;

impl MyContract for Contract {
    fn make_transaction_fail(gas_: u64, amount_: u64, color_: b256, input: u64) -> u64{
        asm(r1: input) {
            rvrt r1;
        };
        42
    }
}
