pub mod contract;
pub mod contract_calls_utils;
pub mod execution_script;
pub mod predicate;

pub mod abi_encoder {
    pub use fuels_core::abi_encoder::*;
}

pub mod abi_decoder {
    pub use fuels_core::abi_decoder::*;
}
