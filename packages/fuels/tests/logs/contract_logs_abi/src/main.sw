library contract_logs_abi;

use std::logging::log;

abi ContractLogs {
    fn produce_logs_values() -> ();
    fn produce_logs_variables() -> ();
    fn produce_logs_custom_types() -> ();
    fn produce_logs_generic_types() -> ();
    fn produce_multiple_logs() -> ();
}
