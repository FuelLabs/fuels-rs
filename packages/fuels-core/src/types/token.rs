use std::fmt;

use crate::types::{
    core::U256,
    errors::{error, Error, Result},
    param_types::EnumVariants,
};

pub type EnumSelector = (u64, Token, EnumVariants);

#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub struct StaticStringToken {
    pub(crate) data: String,
    expected_len: Option<usize>,
}

impl StaticStringToken {
    pub fn new(data: String, expected_len: Option<usize>) -> Self {
        StaticStringToken { data, expected_len }
    }

    fn validate(&self) -> Result<()> {
        if !self.data.is_ascii() {
            return Err(error!(Codec, "string data can only have ascii values"));
        }

        if let Some(expected_len) = self.expected_len {
            if self.data.len() != expected_len {
                return Err(error!(
                    Codec,
                    "string data has len {}, but the expected len is {}",
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
pub enum Token {
    // Used for unit type variants in Enum. An "empty" enum is not represented as Enum<empty box>,
    // because this way we can have both unit and non-unit type variants.
    Unit,
    Bool(bool),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    U128(u128),
    U256(U256),
    B256([u8; 32]),
    Bytes(Vec<u8>),
    String(String),
    RawSlice(Vec<u8>),
    StringArray(StaticStringToken),
    StringSlice(StaticStringToken),
    Tuple(Vec<Token>),
    Array(Vec<Token>),
    Vector(Vec<Token>),
    Struct(Vec<Token>),
    Enum(Box<EnumSelector>),
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
