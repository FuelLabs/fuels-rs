use core::fmt;
use fuel_types::bytes::padded_len;
use strum_macros::EnumString;

pub mod abi_decoder;
pub mod abi_encoder;
pub mod code_gen;
pub mod constants;
pub mod errors;
pub mod json_abi;
pub mod parameters;
pub mod rustfmt;
pub mod source;
pub mod types;
pub mod utils;

pub mod tx {
    #[doc(no_inline)]
    pub use fuel_tx::*;
}

pub type ByteArray = [u8; 8];
pub type Selector = ByteArray;
pub type Bits256 = [u8; 32];
pub type EnumSelector = (u8, Token);

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
    // The Unit paramtype is used for unit variants in Enums. The corresponding type field is `()`,
    // similar to Rust.
    Unit,
    Array(Box<ParamType>, usize),
    #[strum(serialize = "str")]
    String(usize),
    #[strum(disabled)]
    Struct(Vec<ParamType>),
    #[strum(disabled)]
    Enum(Vec<ParamType>),
    Tuple(Vec<ParamType>),
}

impl Default for ParamType {
    fn default() -> Self {
        ParamType::U8
    }
}

pub enum ReturnLocation {
    Return,
    ReturnData,
}

impl ParamType {
    // Depending on the type, the returned value will be stored
    // either in `Return` or `ReturnData`. For more information,
    // see https://github.com/FuelLabs/sway/issues/1368.
    pub fn get_return_location(&self) -> ReturnLocation {
        match &*self {
            Self::Unit | Self::U8 | Self::U16 | Self::U32 | Self::U64 | Self::Bool => {
                ReturnLocation::Return
            }

            _ => ReturnLocation::ReturnData,
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
            ParamType::Enum(inner) => {
                let inner_strings: Vec<String> =
                    inner.iter().map(|p| format!("ParamType::{}", p)).collect();

                let s = format!("Enum(vec![{}])", inner_strings.join(","));
                write!(f, "{}", s)
            }
            ParamType::Tuple(inner) => {
                let inner_strings: Vec<String> =
                    inner.iter().map(|p| format!("ParamType::{}", p)).collect();

                let s = format!("Tuple(vec![{}])", inner_strings.join(","));
                write!(f, "{}", s)
            }
            ParamType::Unit => write! {f, "Unit"},
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
    // Used for unit type variants in Enum. An "empty" enum is not represented as Enum<empty box>,
    // because this way we can have both unit and non-unit type variants.
    Unit,
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
    Tuple(Vec<Token>),
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

// Here we implement `Tokenizable` for a given tuple of a given length.
// This is done this way because we can't use `impl<T> Tokenizable for (T,)`.
// So we implement `Tokenizable` for each tuple length, covering
// a reasonable range of tuple lengths.
macro_rules! impl_tuples {
    ($num: expr, $( $ty: ident : $no: tt, )+) => {
        impl<$($ty, )+> Tokenizable for ($($ty,)+) where
            $(
                $ty: Tokenizable,
            )+
        {
            fn from_token(token: Token) -> Result<Self, InvalidOutputType> {
                match token {
                    Token::Tuple(mut tokens) => {
                        let mut it = tokens.drain(..);
                        Ok(($(
                          $ty::from_token(it.next().expect("All elements are in vector."))?,
                        )+))
                    },
                    other => Err(InvalidOutputType(format!(
                        "Expected `Tuple`, got {:?}",
                        other,
                    ))),
                }
            }

            fn into_token(self) -> Token {
                Token::Tuple(vec![
                    $( self.$no.into_token(), )+
                ])
            }
        }
    }
}

// And where we actually implement the `Tokenizable` for tuples
// from size 1 to size 16.
impl_tuples!(1, A:0, );
impl_tuples!(2, A:0, B:1, );
impl_tuples!(3, A:0, B:1, C:2, );
impl_tuples!(4, A:0, B:1, C:2, D:3, );
impl_tuples!(5, A:0, B:1, C:2, D:3, E:4, );
impl_tuples!(6, A:0, B:1, C:2, D:3, E:4, F:5, );
impl_tuples!(7, A:0, B:1, C:2, D:3, E:4, F:5, G:6, );
impl_tuples!(8, A:0, B:1, C:2, D:3, E:4, F:5, G:6, H:7, );
impl_tuples!(9, A:0, B:1, C:2, D:3, E:4, F:5, G:6, H:7, I:8, );
impl_tuples!(10, A:0, B:1, C:2, D:3, E:4, F:5, G:6, H:7, I:8, J:9, );
impl_tuples!(11, A:0, B:1, C:2, D:3, E:4, F:5, G:6, H:7, I:8, J:9, K:10, );
impl_tuples!(12, A:0, B:1, C:2, D:3, E:4, F:5, G:6, H:7, I:8, J:9, K:10, L:11, );
impl_tuples!(13, A:0, B:1, C:2, D:3, E:4, F:5, G:6, H:7, I:8, J:9, K:10, L:11, M:12, );
impl_tuples!(14, A:0, B:1, C:2, D:3, E:4, F:5, G:6, H:7, I:8, J:9, K:10, L:11, M:12, N:13, );
impl_tuples!(15, A:0, B:1, C:2, D:3, E:4, F:5, G:6, H:7, I:8, J:9, K:10, L:11, M:12, N:13, O:14, );
impl_tuples!(16, A:0, B:1, C:2, D:3, E:4, F:5, G:6, H:7, I:8, J:9, K:10, L:11, M:12, N:13, O:14, P:15, );

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

impl Detokenize for fuel_tx::ContractId {
    fn from_tokens(t: Vec<Token>) -> std::result::Result<Self, InvalidOutputType>
    where
        Self: Sized,
    {
        if let Token::Struct(tokens) = &t[0] {
            if let Token::B256(id) = &tokens[0] {
                Ok(fuel_tx::ContractId::from(*id))
            } else {
                Err(InvalidOutputType(format!(
                    "Expected `b256`, got {:?}",
                    tokens[0]
                )))
            }
        } else {
            Err(InvalidOutputType(format!(
                "Expected `ContractId`, got {:?}",
                t
            )))
        }
    }
}

impl Detokenize for fuel_tx::Address {
    fn from_tokens(t: Vec<Token>) -> std::result::Result<Self, InvalidOutputType>
    where
        Self: Sized,
    {
        if let Token::Struct(tokens) = &t[0] {
            if let Token::B256(id) = &tokens[0] {
                Ok(fuel_tx::Address::from(*id))
            } else {
                Err(InvalidOutputType(format!(
                    "Expected `b256`, got {:?}",
                    tokens[0]
                )))
            }
        } else {
            Err(InvalidOutputType(format!(
                "Expected `Address`, got {:?}",
                t
            )))
        }
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
