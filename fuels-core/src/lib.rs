use core::fmt;
use fuel_types::bytes::padded_len;
use fuel_types::Word;
use strum_macros::EnumString;

pub mod abi_decoder;
pub mod abi_encoder;
pub mod code_gen;
pub mod errors;
pub mod json_abi;
pub mod rustfmt;
pub mod source;
pub mod types;
pub mod utils;

pub type ByteArray = [u8; 8];
pub type Selector = ByteArray;
pub type Bits256 = [u8; 32];
pub type EnumSelector = (u8, Token);
pub const WORD_SIZE: usize = core::mem::size_of::<Word>();
// This constant is used to determine the amount in the 1 UTXO when initializing wallets for now
pub const DEFAULT_COIN_AMOUNT: u64 = 1;
// This constant is the bytes representation of the asset ID of Ethereum right now, the "native"
// token used for gas fees
pub const NATIVE_ASSET_ID: [u8; 32] = [0u8; 32];

#[derive(Debug, Clone, EnumString, PartialEq, Eq)]
#[strum(ascii_case_insensitive)]
pub enum ParamType {
    U8,
    U16,
    U32,
    U64,
    Bool,
    Byte,
    B256,
    Array(Box<ParamType>, usize),
    #[strum(serialize = "str")]
    String(usize),
    #[strum(disabled)]
    Struct(Vec<ParamType>),
    #[strum(disabled)]
    Enum(Vec<ParamType>),
}

impl Default for ParamType {
    fn default() -> Self {
        ParamType::U8
    }
}

impl ParamType {
    // Checks whether the `ParamType` is bigger than a `WORD`
    pub fn bigger_than_word(&self) -> bool {
        match *self {
            Self::B256 => true,
            Self::String(size) => size > 8,
            _ => false,
            // More types will be handled later.
            // Currently, the support for arrays in the SDK is broken
            // due to a change to the array definition in Sway.
        }
    }
}

impl fmt::Display for ParamType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ParamType::String(size) => {
                let t = format!("String({})", size);
                write!(f, "{}", t)
            }
            ParamType::Array(t, size) => {
                let boxed_type_str = format!("Box::new(ParamType::{})", t);
                let arr_str = format!("Array({},{})", boxed_type_str, size);
                write!(f, "{}", arr_str)
            }
            ParamType::Struct(inner) => {
                let inner_strings: Vec<String> =
                    inner.iter().map(|p| format!("ParamType::{}", p)).collect();

                let s = format!("Struct(vec![{}])", inner_strings.join(","));
                write!(f, "{}", s)
            }
            _ => {
                write!(f, "{:?}", self)
            }
        }
    }
}

// Sway types
#[derive(Debug, Clone, PartialEq, EnumString)]
#[strum(ascii_case_insensitive)]
pub enum Token {
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    Bool(bool),
    Byte(u8),
    B256(Bits256),
    Array(Vec<Token>),
    String(String),
    Struct(Vec<Token>),
    Enum(Box<EnumSelector>),
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl<'a> Default for Token {
    fn default() -> Self {
        Token::U8(0)
    }
}

#[derive(Clone, Debug)]
pub struct InvalidOutputType(pub String);

/// Simplified output type for single value.
pub trait Tokenizable {
    /// Converts a `Token` into expected type.
    fn from_token(token: Token) -> Result<Self, InvalidOutputType>
    where
        Self: Sized;
    /// Converts a specified type back into token.
    fn into_token(self) -> Token;
}

impl Tokenizable for Token {
    fn from_token(token: Token) -> Result<Self, InvalidOutputType> {
        Ok(token)
    }
    fn into_token(self) -> Token {
        self
    }
}

impl Tokenizable for bool {
    fn from_token(token: Token) -> Result<Self, InvalidOutputType> {
        match token {
            Token::Bool(data) => Ok(data),
            other => Err(InvalidOutputType(format!(
                "Expected `bool`, got {:?}",
                other
            ))),
        }
    }
    fn into_token(self) -> Token {
        Token::Bool(self)
    }
}

impl Tokenizable for String {
    fn from_token(token: Token) -> Result<Self, InvalidOutputType> {
        match token {
            Token::String(data) => Ok(data),
            other => Err(InvalidOutputType(format!(
                "Expected `String`, got {:?}",
                other
            ))),
        }
    }
    fn into_token(self) -> Token {
        Token::String(self)
    }
}

impl Tokenizable for Bits256 {
    fn from_token(token: Token) -> Result<Self, InvalidOutputType> {
        match token {
            Token::B256(data) => Ok(data),
            other => Err(InvalidOutputType(format!(
                "Expected `String`, got {:?}",
                other
            ))),
        }
    }
    fn into_token(self) -> Token {
        Token::B256(self)
    }
}

impl<T: Tokenizable> Tokenizable for Vec<T> {
    fn from_token(token: Token) -> Result<Self, InvalidOutputType> {
        match token {
            Token::Array(data) => {
                let mut v: Vec<T> = Vec::new();
                for tok in data {
                    v.push(T::from_token(tok.clone()).unwrap());
                }
                Ok(v)
            }
            other => Err(InvalidOutputType(format!("Expected `T`, got {:?}", other))),
        }
    }
    fn into_token(self) -> Token {
        let mut v: Vec<Token> = Vec::new();
        for t in self {
            let tok = T::into_token(t);
            v.push(tok);
        }
        Token::Array(v)
    }
}

impl Tokenizable for u8 {
    fn from_token(token: Token) -> Result<Self, InvalidOutputType> {
        match token {
            Token::U8(data) => Ok(data),
            other => Err(InvalidOutputType(format!("Expected `u8`, got {:?}", other))),
        }
    }
    fn into_token(self) -> Token {
        Token::U8(self)
    }
}

impl Tokenizable for u16 {
    fn from_token(token: Token) -> Result<Self, InvalidOutputType> {
        match token {
            Token::U16(data) => Ok(data),
            other => Err(InvalidOutputType(format!(
                "Expected `u16`, got {:?}",
                other
            ))),
        }
    }
    fn into_token(self) -> Token {
        Token::U16(self)
    }
}

impl Tokenizable for u32 {
    fn from_token(token: Token) -> Result<Self, InvalidOutputType> {
        match token {
            Token::U32(data) => Ok(data),
            other => Err(InvalidOutputType(format!(
                "Expected `u32`, got {:?}",
                other
            ))),
        }
    }
    fn into_token(self) -> Token {
        Token::U32(self)
    }
}

impl Tokenizable for u64 {
    fn from_token(token: Token) -> Result<Self, InvalidOutputType> {
        match token {
            Token::U64(data) => Ok(data),
            other => Err(InvalidOutputType(format!(
                "Expected `u64`, got {:?}",
                other
            ))),
        }
    }
    fn into_token(self) -> Token {
        Token::U64(self)
    }
}

/// Output type possible to deserialize from Contract ABI
pub trait Detokenize {
    /// Creates a new instance from parsed ABI tokens.
    fn from_tokens(tokens: Vec<Token>) -> Result<Self, InvalidOutputType>
    where
        Self: Sized;
}

impl Detokenize for () {
    fn from_tokens(_: Vec<Token>) -> std::result::Result<Self, InvalidOutputType>
    where
        Self: Sized,
    {
        Ok(())
    }
}

impl<T: Tokenizable> Detokenize for T {
    fn from_tokens(mut tokens: Vec<Token>) -> Result<Self, InvalidOutputType> {
        let token = match tokens.len() {
            0 => Token::Struct(vec![]),
            1 => tokens.remove(0),
            _ => Token::Struct(tokens),
        };

        Self::from_token(token)
    }
}

/// Converts a u8 to a right aligned array of 8 bytes.
pub fn pad_u8(value: &u8) -> ByteArray {
    let mut padded = ByteArray::default();
    padded[7] = *value;
    padded
}

/// Converts a u16 to a right aligned array of 8 bytes.
pub fn pad_u16(value: &u16) -> ByteArray {
    let mut padded = ByteArray::default();
    padded[6..].copy_from_slice(&value.to_be_bytes());
    padded
}

/// Converts a u32 to a right aligned array of 8 bytes.
pub fn pad_u32(value: &u32) -> ByteArray {
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
