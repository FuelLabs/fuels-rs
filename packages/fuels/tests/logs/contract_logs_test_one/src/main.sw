contract;

use std::logging::log;
use contract_logs::ContractLogs;

struct TestStruct {
    field_1: bool,
    field_2: b256,
    field_3: u64,
}

impl ContractLogs for Contract {
    fn produce_logs_values() {}

    fn produce_logs_variables() {}
    fn produce_logs_custom_types() {}

    fn produce_logs_generic_types() {}

    fn produce_multiple_logs() {}

    fn produce_logs_bad_abi() {
        let f: u64 = 64;
        let u: b256 = 0xef86afa9696cf0dc6385e2c407a6e159a1103cefb7e2ae0636fb33d3cb2a9e4a;

        let test_struct = TestStruct {
            field_1: true,
            field_2: u,
            field_3: f,
        };

        log(123);
        log(test_struct);
    }
}
