use std::fmt;

use fuel_types::bytes::padded_len;
use strum_macros::EnumString;

use crate::{
    enum_variants::EnumVariants,
    errors::{error, Error, Result},
};

mod bits;
mod bytes;
mod native;
mod raw_slice;
mod sized_ascii_string;

pub use crate::core::{
    bits::*, bytes::Bytes, native::*, raw_slice::RawSlice, sized_ascii_string::*,
};

pub type ByteArray = [u8; 8];
pub type Selector = ByteArray;
pub type EnumSelector = (u8, Token, EnumVariants);

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StringToken {
    data: String,
    expected_len: usize,
}

impl StringToken {
    pub fn new(data: String, expected_len: usize) -> Self {
        StringToken { data, expected_len }
    }

    fn validate(&self) -> Result<()> {
        if !self.data.is_ascii() {
            return Err(error!(
                InvalidData,
                "String data can only have ascii values"
            ));
        }

        if self.data.len() != self.expected_len {
            return Err(error!(
                InvalidData,
                "String data has len {}, but the expected len is {}",
                self.data.len(),
                self.expected_len
            ));
        }

        Ok(())
    }

    pub fn get_encodable_str(&self) -> Result<&str> {
        self.validate()?;
        Ok(self.data.as_str())
    }
}

impl TryFrom<StringToken> for String {
    type Error = Error;
    fn try_from(string_token: StringToken) -> Result<String> {
        string_token.validate()?;
        Ok(string_token.data)
    }
}

#[derive(Debug, Clone, PartialEq, EnumString)]
#[strum(ascii_case_insensitive)]
pub enum Token {
    // Used for unit type variants in Enum. An "empty" enum is not represented as Enum<empty box>,
    // because this way we can have both unit and non-unit type variants.
    Unit,
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    U128(u128),
    Bool(bool),
    B256([u8; 32]),
    Array(Vec<Token>),
    Vector(Vec<Token>),
    String(StringToken),
    Struct(Vec<Token>),
    #[strum(disabled)]
    Enum(Box<EnumSelector>),
    Tuple(Vec<Token>),
    RawSlice(Vec<u64>),
    Bytes(Vec<u8>),
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

/// Converts a u8 to a right aligned array of 8 bytes.
pub fn pad_u8(value: u8) -> ByteArray {
    let mut padded = ByteArray::default();
    padded[7] = value;
    padded
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
