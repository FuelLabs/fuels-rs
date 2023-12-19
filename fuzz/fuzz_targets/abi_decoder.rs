#![no_main]
use fuels::programs::contract::ContractCall;
use fuels_core::{codec::ABIDecoder, types::param_types::ParamType};
use libfuzzer_sys::fuzz_target;

// fuzz_target!(|input: (Vec<ParamType>, &[u8])| {
//     let _ = ABIDecoder::default().decode_multiple(&input.0, input.1);
// });

fuzz_target!(|input: ContractCall| {
    println!("baba");
});
