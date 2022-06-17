use crate::constants::WORD_SIZE;
use core::fmt;
pub use errors::InstantiationError;
use fuel_types::bytes::padded_len;
use std::error::Error;
use strum_macros::EnumString;

pub mod abi_decoder;
pub mod abi_encoder;
pub mod code_gen;
pub mod constants;
mod encoding_utils;
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
pub type EnumSelector = (u8, Token, EnumVariants);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnumVariants {
    variants: Vec<ParamType>,
}

#[derive(Debug)]
pub struct NoVariants;

impl Error for NoVariants {}

impl fmt::Display for NoVariants {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "An Enum must have variants!")
    }
}

impl From<NoVariants> for errors::Error {
    fn from(err: NoVariants) -> Self {
        errors::Error::InvalidType(format!("{}", err))
    }
}

impl EnumVariants {
    pub fn new(variants: Vec<ParamType>) -> Result<EnumVariants, NoVariants> {
        if !variants.is_empty() {
            Ok(EnumVariants { variants })
        } else {
            Err(NoVariants)
        }
    }

    pub fn param_types(&self) -> &Vec<ParamType> {
        &self.variants
    }

    pub fn only_units_inside(&self) -> bool {
        self.variants
            .iter()
            .all(|variant| *variant == ParamType::Unit)
    }
}

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
    Enum(EnumVariants),
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
            ParamType::Enum(variants) => {
                let inner_strings: Vec<String> = variants
                    .param_types()
                    .iter()
                    .map(|p| format!("ParamType::{}", p))
                    .collect();

                let s = format!(
                    "Enum(EnumVariants::new(vec![{}]).unwrap())",
                    inner_strings.join(",")
                );
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
    #[strum(disabled)]
    Enum(Box<EnumSelector>),
    Tuple(Vec<Token>),
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Default for Token {
    fn default() -> Self {
        Token::U8(0)
    }
}

pub trait Tokenizable {
    /// Converts a `Token` into expected type.
    fn from_token(token: Token) -> Result<Self, InstantiationError>
    where
        Self: Sized;
    /// Converts a specified type back into token.
    fn into_token(self) -> Token;
}

impl Tokenizable for Token {
    fn from_token(token: Token) -> Result<Self, InstantiationError> {
        Ok(token)
    }
    fn into_token(self) -> Token {
        self
    }
}

impl Tokenizable for bool {
    fn from_token(token: Token) -> Result<Self, InstantiationError> {
        match token {
            Token::Bool(data) => Ok(data),
            other => Err(InstantiationError(format!(
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
    fn from_token(token: Token) -> Result<Self, InstantiationError> {
        match token {
            Token::String(data) => Ok(data),
            other => Err(InstantiationError(format!(
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
    fn from_token(token: Token) -> Result<Self, InstantiationError> {
        match token {
            Token::B256(data) => Ok(data),
            other => Err(InstantiationError(format!(
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
    fn from_token(token: Token) -> Result<Self, InstantiationError> {
        match token {
            Token::Array(data) => {
                let mut v: Vec<T> = Vec::new();
                for tok in data {
                    v.push(T::from_token(tok.clone()).unwrap());
                }
                Ok(v)
            }
            other => Err(InstantiationError(format!("Expected `T`, got {:?}", other))),
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

impl Tokenizable for () {
    fn from_token(token: Token) -> Result<Self, InstantiationError>
    where
        Self: Sized,
    {
        match token {
            Token::Unit => Ok(()),
            other => Err(InstantiationError(format!(
                "Expected `Unit`, got {:?}",
                other
            ))),
        }
    }

    fn into_token(self) -> Token {
        Token::Unit
    }
}

impl Tokenizable for u8 {
    fn from_token(token: Token) -> Result<Self, InstantiationError> {
        match token {
            Token::U8(data) => Ok(data),
            other => Err(InstantiationError(format!(
                "Expected `u8`, got {:?}",
                other
            ))),
        }
    }
    fn into_token(self) -> Token {
        Token::U8(self)
    }
}

impl Tokenizable for u16 {
    fn from_token(token: Token) -> Result<Self, InstantiationError> {
        match token {
            Token::U16(data) => Ok(data),
            other => Err(InstantiationError(format!(
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
    fn from_token(token: Token) -> Result<Self, InstantiationError> {
        match token {
            Token::U32(data) => Ok(data),
            other => Err(InstantiationError(format!(
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
    fn from_token(token: Token) -> Result<Self, InstantiationError> {
        match token {
            Token::U64(data) => Ok(data),
            other => Err(InstantiationError(format!(
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
            fn from_token(token: Token) -> Result<Self, InstantiationError> {
                match token {
                    Token::Tuple(tokens) => {
                        let mut it = tokens.into_iter();
                        let mut next_token = move || {
                            it.next().ok_or_else(|| {
                                InstantiationError("Ran out of tokens before tuple could be constructed".to_string())
                            })
                        };
                        Ok(($(
                          $ty::from_token(next_token()?)?,
                        )+))
                    },
                    other => Err(InstantiationError(format!(
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

impl Tokenizable for fuel_tx::ContractId {
    fn from_token(token: Token) -> std::result::Result<Self, InstantiationError>
    where
        Self: Sized,
    {
        if let Token::Struct(tokens) = token {
            let first_token = tokens.into_iter().next();
            if let Some(Token::B256(id)) = first_token {
                Ok(fuel_tx::ContractId::from(id))
            } else {
                Err(InstantiationError(format!(
                    "Expected `b256`, got {:?}",
                    first_token
                )))
            }
        } else {
            Err(InstantiationError(format!(
                "Expected `ContractId`, got {:?}",
                token
            )))
        }
    }

    fn into_token(self) -> Token {
        let underlying_data: &Bits256 = &self;
        Token::Struct(vec![underlying_data.into_token()])
    }
}

impl Tokenizable for fuel_tx::Address {
    fn from_token(t: Token) -> std::result::Result<Self, InstantiationError>
    where
        Self: Sized,
    {
        if let Token::Struct(tokens) = t {
            let first_token = tokens.into_iter().next();
            if let Some(Token::B256(id)) = first_token {
                Ok(fuel_tx::Address::from(id))
            } else {
                Err(InstantiationError(format!(
                    "Expected `b256`, got {:?}",
                    first_token
                )))
            }
        } else {
            Err(InstantiationError(format!(
                "Expected `Address`, got {:?}",
                t
            )))
        }
    }

    fn into_token(self) -> Token {
        let underlying_data: &Bits256 = &self;

        Token::Struct(vec![underlying_data.into_token()])
    }
}

impl Tokenizable for fuel_tx::AssetId {
    fn from_token(token: Token) -> Result<Self, InstantiationError>
    where
        Self: Sized,
    {
        if let Token::Struct(inner_tokens) = token {
            let first_token = inner_tokens.into_iter().next();
            if let Some(Token::B256(id)) = first_token {
                Ok(Self::from(id))
            } else {
                Err(InstantiationError(format!("Could not construct 'AssetId' from token. Wrong token inside of Struct '{:?} instead of B256'", first_token)))
            }
        } else {
            Err(InstantiationError(format!("Could not construct 'AssetId' from token. Instead of a Struct with a B256 inside, received: {:?}", token)))
        }
    }

    fn into_token(self) -> Token {
        let underlying_data: &Bits256 = &self;
        Token::Struct(vec![underlying_data.into_token()])
    }
}

/// This trait is used inside the abigen generated code in order to get the
/// parameter types (`ParamType`).  This is used in the generated code in
/// `custom_types_gen.rs`, with the exception of the Sway-native types
/// `Address`, `ContractId`, and `AssetId`, that are implemented right here,
/// without code generation.
pub trait Parameterize {
    fn param_types() -> Vec<ParamType>;
}

impl Parameterize for fuel_tx::Address {
    fn param_types() -> Vec<ParamType> {
        vec![ParamType::B256]
    }
}

impl Parameterize for fuel_tx::ContractId {
    fn param_types() -> Vec<ParamType> {
        vec![ParamType::B256]
    }
}

impl Parameterize for fuel_tx::AssetId {
    fn param_types() -> Vec<ParamType> {
        vec![ParamType::B256]
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
