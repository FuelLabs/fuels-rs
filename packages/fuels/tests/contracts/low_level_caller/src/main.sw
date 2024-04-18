contract;

use std::{
    bytes::Bytes,
    constants::ZERO_B256,
    low_level_call::{
        call_with_function_selector,
        CallParams,
    },
};

abi MyCallerContract {
    fn call_low_level_call(
        target: ContractId,
        function_selector: Bytes,
        calldata: Bytes,
    );
}

impl MyCallerContract for Contract {
    // ANCHOR: low_level_call_contract
    fn call_low_level_call(
        target: ContractId,
        function_selector: Bytes,
        calldata: Bytes,
    ) {
        let call_params = CallParams {
            coins: 0,
            asset_id: AssetId::from(ZERO_B256),
            gas: 10_000,
        };

        call_with_function_selector(target, function_selector, calldata, call_params);
    }
    // ANCHOR_END: low_level_call_contract
}
