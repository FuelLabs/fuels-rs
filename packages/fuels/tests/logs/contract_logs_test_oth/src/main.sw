contract;

use std::logging::log;
use contract_logs::ContractLogs;

impl ContractLogs for Contract {
    fn produce_logs_values() {}

    fn produce_logs_variables() {}
    fn produce_logs_custom_types() {}

    fn produce_logs_generic_types() {}

    fn produce_multiple_logs() {}

    fn produce_logs_bad_abi() {
        log(123);
    }
}
