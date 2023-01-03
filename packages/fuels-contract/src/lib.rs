use crate::logs::LogDecoder;
use fuels_types::bech32::Bech32ContractId;

pub mod call_response;
pub mod contract;
pub mod contract_calls_utils;
pub mod execution_script;
pub mod logs;
pub mod script_calls;

pub mod abi_encoder {
    pub use fuels_core::abi_encoder::*;
}

pub mod abi_decoder {
    pub use fuels_core::abi_decoder::*;
}

pub trait SetableContract {
    fn id(&self) -> Bech32ContractId;
    fn log_decoder(&self) -> LogDecoder;
}
