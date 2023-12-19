#![no_main]
use fuels::{
    programs::contract::{CallParameters, ContractCall},
    types::{bech32::Bech32ContractId, unresolved_bytes::UnresolvedBytes, AssetId, Bytes32},
};
use libfuzzer_sys::{arbitrary::Arbitrary, fuzz_target};

// fuzz_target!(|input: (Vec<ParamType>, &[u8])| {
//     let _ = ABIDecoder::default().decode_multiple(&input.0, input.1);
// });

fuzz_target!(|input: ContractCall| {
    println!("baba");
});
