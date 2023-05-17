contract;

use std::{bytes::Bytes, constants::BASE_ASSET_ID, low_level_call::CallParams};

abi Caller {
    fn general_call(callee_contract_id: ContractId, calldata: Bytes);
}

impl Caller for Contract {
    fn general_call(callee_contract_id: ContractId, calldata: Bytes) {
        let call_params = CallParams {
            coins: 0,
            asset_id: BASE_ASSET_ID,
            gas: 10_000_000,
        };
        let payload = create_payload(callee_contract_id, calldata);
        call_with_raw_payload(payload, call_params);
    }
}

/// Call a target contract with an already-encoded payload.
/// `payload` : The encoded payload to be called.
fn call_with_raw_payload(payload: Bytes, call_params: CallParams) {
    asm(r1: payload.buf.ptr, r2: call_params.coins, r3: call_params.asset_id, r4: call_params.gas) {
        call r1 r2 r3 r4;
    };
}

/// Encode a payload from the callee_contract_id and calldata.
fn create_payload(target: ContractId, calldata: Bytes) -> Bytes {
    /*
    packs args according to spec (https://github.com/FuelLabs/fuel-specs/blob/master/src/vm/instruction_set.md#call-call-contract) :

    bytes   type        value   description
    32	    byte[32]    to      Contract ID to call.
    8	    byte[8]	    param1  First parameter (function selector).
    8	    byte[8]	    param2  Second parameter (abi-encoded calldata: value if value type, otherwise pointer to reference type).
    */
    let mut payload = Bytes::new();
    payload.append(contract_id_to_bytes(target));
    payload.append(calldata);

    payload
}

/// Represent a contract ID as a `Bytes`, so it can be concatenated with a payload.
fn contract_id_to_bytes(contract_id: ContractId) -> Bytes {
    let mut target_bytes = Bytes::with_capacity(32);
    target_bytes.len = 32;

    __addr_of(contract_id).copy_bytes_to(target_bytes.buf.ptr, 32);

    target_bytes
}
