use crate::abi_decoder::ABIDecoder;
use core::fmt;
use fuel_types::bytes::padded_len;
use fuels_types::{
    errors::{CodecError, Error},
    param_types::{EnumVariants, ParamType},
};
use strum_macros::EnumString;

pub mod abi_decoder;
pub mod abi_encoder;
pub mod code_gen;
pub mod constants;
pub mod json_abi;
pub mod parameters;
pub mod rustfmt;
pub mod source;
pub mod tokenizer;
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

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StringToken {
    data: String,
    expected_len: usize,
}

impl StringToken {
    pub fn new(data: String, expected_len: usize) -> Self {
        StringToken { data, expected_len }
    }

    pub fn get_encodable_str(&self) -> Result<&str, CodecError> {
        if !self.data.is_ascii() {
            return Err(CodecError::InvalidData(
                "String data can only have ascii values".into(),
            ));
        }

        if self.data.len() != self.expected_len {
            return Err(CodecError::InvalidData(format!(
                "String data has len {}, but the expected len is {}",
                self.data.len(),
                self.expected_len
            )));
        }
        Ok(self.data.as_str())
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
    String(StringToken),
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
    fn from_token(token: Token) -> Result<Self, Error>
    where
        Self: Sized;
    /// Converts a specified type back into token.
    fn into_token(self) -> Token;
}

pub fn try_from_bytes<T>(bytes: &[u8]) -> Result<T, Error>
where
    T: Parameterize + Tokenizable,
{
    let token = ABIDecoder::decode_single(&T::param_type(), bytes)?;

    T::from_token(token)
}

impl Tokenizable for Token {
    fn from_token(token: Token) -> Result<Self, Error> {
        Ok(token)
    }
    fn into_token(self) -> Token {
        self
    }
}

impl Tokenizable for bool {
    fn from_token(token: Token) -> Result<Self, Error> {
        match token {
            Token::Bool(data) => Ok(data),
            other => Err(Error::InstantiationError(format!(
                "Expected `bool`, got {:?}",
                other
            ))),
        }
    }
    fn into_token(self) -> Token {
        Token::Bool(self)
    }
}

impl Tokenizable for StringToken {
    fn from_token(token: Token) -> Result<Self, Error> {
        match token {
            Token::String(string_token @ StringToken { .. }) => Ok(string_token),
            other => Err(Error::InstantiationError(format!(
                "Expected `String`, got {:?}",
                other
            ))),
        }
    }
    fn into_token(self) -> Token {
        Token::String(self)
    }
}

impl Tokenizable for String {
    fn from_token(token: Token) -> Result<Self, Error> {
        match token {
            Token::String(string_token) => Ok(string_token.data),
            other => Err(Error::InstantiationError(format!(
                "Expected `String`, got {:?}",
                other
            ))),
        }
    }
    fn into_token(self) -> Token {
        let len = self.len();
        Token::String(StringToken::new(self, len))
    }
}

impl Tokenizable for Bits256 {
    fn from_token(token: Token) -> Result<Self, Error> {
        match token {
            Token::B256(data) => Ok(data),
            other => Err(Error::InstantiationError(format!(
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
    fn from_token(token: Token) -> Result<Self, Error> {
        match token {
            Token::Array(data) => {
                let mut v: Vec<T> = Vec::new();
                for tok in data {
                    v.push(T::from_token(tok.clone()).unwrap());
                }
                Ok(v)
            }
            other => Err(Error::InstantiationError(format!(
                "Expected `T`, got {:?}",
                other
            ))),
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
    fn from_token(token: Token) -> Result<Self, Error>
    where
        Self: Sized,
    {
        match token {
            Token::Unit => Ok(()),
            other => Err(Error::InstantiationError(format!(
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
    fn from_token(token: Token) -> Result<Self, Error> {
        match token {
            Token::U8(data) => Ok(data),
            other => Err(Error::InstantiationError(format!(
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
    fn from_token(token: Token) -> Result<Self, Error> {
        match token {
            Token::U16(data) => Ok(data),
            other => Err(Error::InstantiationError(format!(
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
    fn from_token(token: Token) -> Result<Self, Error> {
        match token {
            Token::U32(data) => Ok(data),
            other => Err(Error::InstantiationError(format!(
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
    fn from_token(token: Token) -> Result<Self, Error> {
        match token {
            Token::U64(data) => Ok(data),
            other => Err(Error::InstantiationError(format!(
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
            fn from_token(token: Token) -> Result<Self, Error> {
                match token {
                    Token::Tuple(tokens) => {
                        let mut it = tokens.into_iter();
                        let mut next_token = move || {
                            it.next().ok_or_else(|| {
                                Error::InstantiationError("Ran out of tokens before tuple could be constructed".to_string())
                            })
                        };
                        Ok(($(
                          $ty::from_token(next_token()?)?,
                        )+))
                    },
                    other => Err(Error::InstantiationError(format!(
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

        impl<$($ty, )+> Parameterize for ($($ty,)+) where
            $(
                $ty: Parameterize,
            )+
        {
            fn param_type() -> ParamType {
                ParamType::Tuple(vec![
                    $( $ty::param_type(), )+
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
    fn from_token(token: Token) -> std::result::Result<Self, Error>
    where
        Self: Sized,
    {
        if let Token::Struct(tokens) = token {
            let first_token = tokens.into_iter().next();
            if let Some(Token::B256(id)) = first_token {
                Ok(fuel_tx::ContractId::from(id))
            } else {
                Err(Error::InstantiationError(format!(
                    "Expected `b256`, got {:?}",
                    first_token
                )))
            }
        } else {
            Err(Error::InstantiationError(format!(
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
    fn from_token(t: Token) -> std::result::Result<Self, Error>
    where
        Self: Sized,
    {
        if let Token::Struct(tokens) = t {
            let first_token = tokens.into_iter().next();
            if let Some(Token::B256(id)) = first_token {
                Ok(fuel_tx::Address::from(id))
            } else {
                Err(Error::InstantiationError(format!(
                    "Expected `b256`, got {:?}",
                    first_token
                )))
            }
        } else {
            Err(Error::InstantiationError(format!(
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
    fn from_token(token: Token) -> Result<Self, Error>
    where
        Self: Sized,
    {
        if let Token::Struct(inner_tokens) = token {
            let first_token = inner_tokens.into_iter().next();
            if let Some(Token::B256(id)) = first_token {
                Ok(Self::from(id))
            } else {
                Err(Error::InstantiationError(format!("Could not construct 'AssetId' from token. Wrong token inside of Struct '{:?} instead of B256'", first_token)))
            }
        } else {
            Err(Error::InstantiationError(format!("Could not construct 'AssetId' from token. Instead of a Struct with a B256 inside, received: {:?}", token)))
        }
    }

    fn into_token(self) -> Token {
        let underlying_data: &Bits256 = &self;
        Token::Struct(vec![underlying_data.into_token()])
    }
}

/// `abigen` requires `Parameterized` to construct nested types. It is also used by `try_from_bytes`
/// to facilitate the instantiation of custom types from bytes.
pub trait Parameterize {
    fn param_type() -> ParamType;
}

impl Parameterize for fuel_tx::Address {
    fn param_type() -> ParamType {
        ParamType::Struct(vec![ParamType::B256])
    }
}

impl Parameterize for fuel_tx::ContractId {
    fn param_type() -> ParamType {
        ParamType::Struct(vec![ParamType::B256])
    }
}

impl Parameterize for fuel_tx::AssetId {
    fn param_type() -> ParamType {
        ParamType::Struct(vec![ParamType::B256])
    }
}

impl Parameterize for () {
    fn param_type() -> ParamType {
        ParamType::Unit
    }
}

impl Parameterize for bool {
    fn param_type() -> ParamType {
        ParamType::Bool
    }
}

impl Parameterize for u8 {
    fn param_type() -> ParamType {
        ParamType::U8
    }
}

impl Parameterize for u16 {
    fn param_type() -> ParamType {
        ParamType::U16
    }
}

impl Parameterize for u32 {
    fn param_type() -> ParamType {
        ParamType::U32
    }
}

impl Parameterize for u64 {
    fn param_type() -> ParamType {
        ParamType::U64
    }
}

impl Parameterize for Bits256 {
    fn param_type() -> ParamType {
        ParamType::B256
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

#[cfg(test)]
mod tests {
    use crate::try_from_bytes;
    use fuel_types::{Address, AssetId, ContractId};
    use fuels_types::{constants::WORD_SIZE, errors::Error};

    #[test]
    fn can_convert_bytes_into_tuple() -> Result<(), Error> {
        let tuple_in_bytes: Vec<u8> = vec![0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 2];

        let the_tuple: (u64, u32) = try_from_bytes(&tuple_in_bytes)?;

        assert_eq!(the_tuple, (1, 2));

        Ok(())
    }

    #[test]
    fn can_convert_all_from_bool_to_u64() -> Result<(), Error> {
        let bytes: Vec<u8> = vec![0xFF; WORD_SIZE];

        assert!(try_from_bytes::<bool>(&bytes)?);
        assert_eq!(try_from_bytes::<u8>(&bytes)?, u8::MAX);
        assert_eq!(try_from_bytes::<u16>(&bytes)?, u16::MAX);
        assert_eq!(try_from_bytes::<u32>(&bytes)?, u32::MAX);
        assert_eq!(try_from_bytes::<u64>(&bytes)?, u64::MAX);

        Ok(())
    }

    #[test]
    fn can_convert_native_types() -> anyhow::Result<()> {
        let bytes = [0xFF; 32];

        assert_eq!(
            try_from_bytes::<Address>(&bytes)?,
            Address::new(bytes.as_slice().try_into()?)
        );
        assert_eq!(
            try_from_bytes::<ContractId>(&bytes)?,
            ContractId::new(bytes.as_slice().try_into()?)
        );
        assert_eq!(
            try_from_bytes::<AssetId>(&bytes)?,
            AssetId::new(bytes.as_slice().try_into()?)
        );
        Ok(())
    }
}
