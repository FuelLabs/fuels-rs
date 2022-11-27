use crate::{abi_decoder::ABIDecoder, types::Bits256};
use fuel_types::bytes::padded_len;
use fuels_types::{
    enum_variants::EnumVariants,
    errors::{CodecError, Error},
    param_types::ParamType,
};
use std::{fmt, iter::zip};
use strum_macros::EnumString;

pub mod abi_decoder;
pub mod abi_encoder;
pub mod code_gen;
pub mod constants;
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
pub type EnumSelector = (u8, Token, EnumVariants);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Identity {
    Address(fuel_tx::Address),
    ContractId(fuel_tx::ContractId),
}

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
    B256([u8; 32]),
    Array(Vec<Token>),
    Vector(Vec<Token>),
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

/// `abigen` requires `Parameterized` to construct nested types. It is also used by `try_from_bytes`
/// to facilitate the instantiation of custom types from bytes.
pub trait Parameterize {
    fn param_type() -> ParamType;
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

pub fn unzip_param_types(param_types: &[(String, ParamType)]) -> Vec<ParamType> {
    param_types
        .iter()
        .map(|(_, param_type)| param_type)
        .cloned()
        .collect()
}

pub trait DecodableLog {
    fn decode_log(&self, data: &[u8]) -> Result<String, Error>;
}

impl DecodableLog for ParamType {
    fn decode_log(&self, data: &[u8]) -> Result<String, Error> {
        let token = ABIDecoder::decode_single(self, data)?;
        paramtype_decode_log(self, &token)
    }
}

fn inner_types_log(
    tokens: &[Token],
    inner_type: &ParamType,
    join_str: &str,
) -> Result<String, Error> {
    let inner_types_log = tokens
        .iter()
        .map(|token| paramtype_decode_log(inner_type, token))
        .collect::<Result<Vec<_>, _>>()?
        .join(join_str);

    Ok(inner_types_log)
}

fn paramtype_decode_log(param_type: &ParamType, token: &Token) -> Result<String, Error> {
    let result = match (param_type, token) {
        (ParamType::U8, Token::U8(val)) => val.to_string(),
        (ParamType::U16, Token::U16(val)) => val.to_string(),
        (ParamType::U32, Token::U32(val)) => val.to_string(),
        (ParamType::U64, Token::U64(val)) => val.to_string(),
        (ParamType::Bool, Token::Bool(val)) => val.to_string(),
        (ParamType::Byte, Token::Byte(val)) => val.to_string(),
        (ParamType::B256, Token::B256(val)) => {
            format!("Bits256({val:?})")
        }
        (ParamType::Unit, Token::Unit) => "()".to_string(),
        (ParamType::String(..), Token::String(str_token)) => {
            format!("SizedAsciiString {{ data: \"{}\" }}", str_token.data)
        }
        (ParamType::Array(inner_type, _), Token::Array(tokens)) => {
            let elements = inner_types_log(tokens, inner_type, ", ")?;
            format!("[{elements}]")
        }
        (ParamType::Vector(inner_type), Token::Vector(tokens)) => {
            let elements = inner_types_log(tokens, inner_type, ", ")?;
            format!("[{elements}]")
        }
        (ParamType::Struct { name, fields, .. }, Token::Struct(field_tokens)) => {
            let fields = zip(fields, field_tokens)
                .map(|((field_name, param_type), token)| -> Result<_, Error> {
                    let field_stringified = paramtype_decode_log(param_type, token)?;
                    Ok(format!("{field_name}: {}", field_stringified))
                })
                .collect::<Result<Vec<_>, _>>()?
                .join(", ");
            format!("{name} {{ {fields} }}")
        }
        (ParamType::Enum { .. }, Token::Enum(selector)) => {
            let (discriminant, token, variants) = selector.as_ref();

            let (variant_name, variant_param_type) = variants.select_variant(*discriminant)?;
            let variant_str = paramtype_decode_log(variant_param_type, token)?;
            let variant_str = if variant_str == "()" {
                "".into()
            } else {
                format!("({variant_str})")
            };

            format!("{variant_name}{variant_str}")
        }
        (ParamType::Tuple(types), Token::Tuple(tokens)) => {
            let elements = zip(types, tokens)
                .map(|(ptype, token)| paramtype_decode_log(ptype, token))
                .collect::<Result<Vec<_>, _>>()?
                .join(", ");

            format!("({elements})")
        }
        _ => {
            return Err(Error::InvalidData(format!(
                "Could not decode log with param type: `{param_type:?}` and token: `{token:?}`"
            )))
        }
    };
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::DecodableLog;
    use crate::{
        try_from_bytes,
        types::{Bits256, EvmAddress, SizedAsciiString},
        Parameterize,
    };
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

    #[test]
    fn test_param_type_decode_log() -> anyhow::Result<()> {
        {
            assert_eq!(
                format!("{:?}", true),
                bool::param_type().decode_log(&[0, 0, 0, 0, 0, 0, 0, 1])?
            );

            assert_eq!(
                format!("{:?}", 128u8),
                u8::param_type().decode_log(&[0, 0, 0, 0, 0, 0, 0, 128])?
            );

            assert_eq!(
                format!("{:?}", 256u16),
                u16::param_type().decode_log(&[0, 0, 0, 0, 0, 0, 1, 0])?
            );

            assert_eq!(
                format!("{:?}", 512u32),
                u32::param_type().decode_log(&[0, 0, 0, 0, 0, 0, 2, 0])?
            );

            assert_eq!(
                format!("{:?}", 1024u64),
                u64::param_type().decode_log(&[0, 0, 0, 0, 0, 0, 4, 0])?
            );
        }
        {
            assert_eq!(
                format!("{:?}", (1, 2)),
                <(u8, u8)>::param_type()
                    .decode_log(&[0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 2])?
            );

            assert_eq!(
                format!("{:?}", [3, 4]),
                <[u64; 2]>::param_type()
                    .decode_log(&[0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 4])?
            );

            assert_eq!(
                format!("{:?}", SizedAsciiString::<4>::new("Fuel".to_string())?),
                SizedAsciiString::<4>::param_type().decode_log(&[70, 117, 101, 108, 0, 0, 0, 0])?
            );
        }
        {
            assert_eq!(
                format!("{:?}", Some(42)),
                <Option<u64>>::param_type()
                    .decode_log(&[0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 42])?
            );

            assert_eq!(
                format!("{:?}", Err::<u64, u64>(42u64)),
                <Result<u64, u64>>::param_type()
                    .decode_log(&[0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 42])?
            );

            let bits256 = Bits256([
                239, 134, 175, 169, 105, 108, 240, 220, 99, 133, 226, 196, 7, 166, 225, 89, 161,
                16, 60, 239, 183, 226, 174, 6, 54, 251, 51, 211, 203, 42, 158, 74,
            ]);

            assert_eq!(
                format!("{:?}", bits256),
                Bits256::param_type().decode_log(&[
                    239, 134, 175, 169, 105, 108, 240, 220, 99, 133, 226, 196, 7, 166, 225, 89,
                    161, 16, 60, 239, 183, 226, 174, 6, 54, 251, 51, 211, 203, 42, 158, 74
                ])?
            );

            assert_eq!(
                format!("{:?}", EvmAddress::from(bits256)),
                EvmAddress::param_type().decode_log(&[
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 7, 166, 225, 89, 161, 16, 60, 239, 183,
                    226, 174, 6, 54, 251, 51, 211, 203, 42, 158, 74
                ])?
            );
        }

        Ok(())
    }
}
