use fuel_types::bytes::padded_len;
pub use fuel_types::{
    Address, AssetId, BlockHeight, Bytes32, Bytes4, Bytes64, Bytes8, ChainId, ContractId,
    MessageId, Nonce, Salt, Word,
};

pub use crate::types::{core::*, token::*, wrappers::*};

pub mod bech32;
mod core;
pub mod errors;
pub mod param_types;
mod token;
pub mod transaction_builders;
pub mod tx_status;
pub mod unresolved_bytes;
mod wrappers;

pub type ByteArray = [u8; 8];
#[cfg(not(feature = "experimental"))]
pub type Selector = ByteArray;
#[cfg(feature = "experimental")]
pub type Selector = Vec<u8>;

/// Converts a u16 to a right aligned array of 8 bytes.
pub fn pad_u16(value: u16) -> ByteArray {
    let mut padded = ByteArray::default();
    padded[6..].copy_from_slice(&value.to_be_bytes());
    padded
}

/// Converts a u32 to a right aligned array of 8 bytes.
pub fn pad_u32(value: u32) -> ByteArray {
    let mut padded = [0u8; 8];
    padded[4..].copy_from_slice(&value.to_be_bytes());
    padded
}

pub fn pad_string(s: &str) -> Vec<u8> {
    let pad = padded_len(s.as_bytes()) - s.len();

    let mut padded = s.as_bytes().to_owned();

    padded.extend_from_slice(&vec![0; pad]);

    padded
}
