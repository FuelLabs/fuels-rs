use crate::abi_decoder::ABIDecoder;
use crate::types::Bits256;
use core::fmt;
use fuel_types::bytes::padded_len;
use fuels_types::enum_variants::EnumVariants;
use fuels_types::{
    errors::{CodecError, Error},
    param_types::ParamType,
    ABIFunction, LoggedType, TypeApplication, TypeDeclaration,
};
use std::collections::HashMap;
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

#[derive(Debug, Clone)]
pub struct FullABIFunction {
    pub inputs: Vec<FullTypeApplication>,
    pub name: String,
    pub output: FullTypeApplication,
}
impl FullABIFunction {
    pub fn from_abi_function(
        abi_function: &ABIFunction,
        types: &HashMap<usize, TypeDeclaration>,
    ) -> FullABIFunction {
        let inputs = abi_function
            .inputs
            .iter()
            .map(|input| FullTypeApplication::from_type_application(input, types))
            .collect();
        FullABIFunction {
            inputs,
            name: abi_function.name.clone(),
            output: FullTypeApplication::from_type_application(&abi_function.output, types),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FullTypeDeclaration {
    pub type_field: String,
    pub components: Vec<FullTypeApplication>,
    pub type_parameters: Vec<FullTypeDeclaration>,
}

impl FullTypeDeclaration {
    pub fn from_type_declaration(
        type_decl: &TypeDeclaration,
        types: &HashMap<usize, TypeDeclaration>,
    ) -> FullTypeDeclaration {
        let components = type_decl
            .components
            .clone()
            .unwrap_or_default()
            .into_iter()
            .map(|application| FullTypeApplication::from_type_application(&application, types))
            .collect();
        let type_parameters = type_decl
            .type_parameters
            .clone()
            .unwrap_or_default()
            .into_iter()
            .map(|id| FullTypeDeclaration::from_type_declaration(types.get(&id).unwrap(), types))
            .collect();
        FullTypeDeclaration {
            type_field: type_decl.type_field.clone(),
            components,
            type_parameters,
        }
    }
}
impl FullTypeApplication {
    pub fn from_type_application(
        type_application: &TypeApplication,
        types: &HashMap<usize, TypeDeclaration>,
    ) -> FullTypeApplication {
        let type_arguments = type_application
            .type_arguments
            .clone()
            .unwrap_or_default()
            .into_iter()
            .map(|application| FullTypeApplication::from_type_application(&application, types))
            .collect();

        let type_decl = FullTypeDeclaration::from_type_declaration(
            types.get(&type_application.type_id).unwrap(),
            types,
        );

        FullTypeApplication {
            name: type_application.name.clone(),
            type_decl,
            type_arguments,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FullTypeApplication {
    pub name: String,
    pub type_decl: FullTypeDeclaration,
    pub type_arguments: Vec<FullTypeApplication>,
}

#[derive(Debug, Clone)]
pub struct FullLoggedType {
    pub log_id: u64,
    pub application: FullTypeApplication,
}

impl FullLoggedType {
    pub fn from_logged_type(
        logged_type: &LoggedType,
        types: &HashMap<usize, TypeDeclaration>,
    ) -> FullLoggedType {
        FullLoggedType {
            log_id: logged_type.log_id,
            application: FullTypeApplication::from_type_application(
                &logged_type.application,
                types,
            ),
        }
    }
}

impl FullTypeDeclaration {
    pub fn is_enum_type(&self) -> bool {
        let type_field = &self.type_field;
        type_field.starts_with("enum ")
    }

    pub fn is_struct_type(&self) -> bool {
        let type_field = &self.type_field;
        type_field.starts_with("struct ")
    }
}
