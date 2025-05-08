script;

use std::logging::log;
use library::ContractLogs;

#[allow(dead_code)]
enum MatchEnum {
    Logs: (),
    Panic: (),
    PanicError: (),
}

fn main(contract_id: ContractId, match_enum: MatchEnum) {
    let contract_instance = abi(ContractLogs, contract_id.into());
    match match_enum {
        MatchEnum::Logs => {
            contract_instance.produce_logs_values();

            let f: bool = true;
            let u: u64 = 42;
            let e: str[4] = __to_str_array("Fuel");
            let l: [u8; 3] = [1u8, 2u8, 3u8];
            log(f);
            log(u);
            log(e);
            log(l);
        }
        MatchEnum::Panic => {
            contract_instance.produce_panic();
        }
        MatchEnum::PanicError => {
            contract_instance.produce_panic_with_error();
        }
    }
}
