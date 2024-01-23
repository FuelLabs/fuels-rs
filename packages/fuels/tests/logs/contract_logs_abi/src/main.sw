library;

use std::logging::log;

abi ContractLogs {
    fn produce_logs_values();
    fn produce_logs_variables();
    fn produce_logs_custom_types();
    fn produce_logs_generic_types();
    fn produce_multiple_logs();
    fn produce_bad_logs();
    fn produce_string_slice_log();
    fn produce_string_log();
    fn produce_bytes_log();
    fn produce_raw_slice_log();
    fn produce_vec_log();
}
