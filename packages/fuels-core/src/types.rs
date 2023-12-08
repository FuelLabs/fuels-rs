use std::fmt;

pub use fuel_tx::{Address, AssetId, ContractId, TxPointer, UtxoId};
use fuel_types::bytes::padded_len;
pub use fuel_types::{ChainId, MessageId, Nonce};

pub use crate::types::{core::*, wrappers::*};
use crate::types::{
    enum_variants::EnumVariants,
    errors::{error, Error, Result},
};

pub mod bech32;
mod core;
pub mod enum_variants;
pub mod errors;
pub mod param_types;
pub mod transaction_builders;
pub mod tx_status;
pub mod unresolved_bytes;
mod wrappers;

pub type ByteArray = [u8; 8];
pub type Selector = ByteArray;
pub type EnumSelector = (u64, Token, EnumVariants);

#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub struct StaticStringToken {
    data: String,
    expected_len: Option<usize>,
}

impl StaticStringToken {
    pub fn new(data: String, expected_len: Option<usize>) -> Self {
        StaticStringToken { data, expected_len }
    }

    fn validate(&self) -> Result<()> {
        if !self.data.is_ascii() {
            return Err(error!(
                InvalidData,
                "String data can only have ascii values"
            ));
        }

        if let Some(expected_len) = self.expected_len {
            if self.data.len() != expected_len {
                return Err(error!(
                    InvalidData,
                    "String data has len {}, but the expected len is {}",
                    self.data.len(),
                    expected_len
                ));
            }
        }

        Ok(())
    }

    pub fn get_encodable_str(&self) -> Result<&str> {
        self.validate()?;
        Ok(self.data.as_str())
    }
}

impl TryFrom<StaticStringToken> for String {
    type Error = Error;
    fn try_from(string_token: StaticStringToken) -> Result<String> {
        string_token.validate()?;
        Ok(string_token.data)
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub enum Token {
    // Used for unit type variants in Enum. An "empty" enum is not represented as Enum<empty box>,
    // because this way we can have both unit and non-unit type variants.
    Unit,
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    U128(u128),
    U256(U256),
    Bool(bool),
    B256([u8; 32]),
    Array(Vec<Token>),
    Vector(Vec<Token>),
    StringSlice(StaticStringToken),
    StringArray(StaticStringToken),
    Struct(Vec<Token>),
    Enum(Box<EnumSelector>),
    Tuple(Vec<Token>),
    RawSlice(Vec<u64>),
    Bytes(Vec<u8>),
    String(String),
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl Default for Token {
    fn default() -> Self {
        Token::U8(0)
    }
}

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
