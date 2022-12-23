script;

use std::logging::log;
use library::ContractLogs;

fn main(contract_id: ContractId) {
    // Call contract with `contract_id` and make some logs
    let contract_instance = abi(ContractLogs, contract_id.into());
    contract_instance.produce_logs_values();

    let f: bool = true;
    let u: u64 = 42;
    let e: str[4] = "Fuel";
    let l: [u8; 3] = [1u8, 2u8, 3u8];
    log(f);
    log(u);
    log(e);
    log(l);
}
