//! Defines a set of serializable types required for the Fuel VM ABI.
//!
//! We declare these in a dedicated, minimal crate in order to allow for downstream projects to
//! consume or generate these ABI-compatible types without needing to pull in the rest of the SDK.

use serde::{Deserialize, Serialize};
use strum_macros::ToString;

pub mod bech32;
pub mod constants;
pub mod errors;
pub mod param_types;
pub mod parse_param;
pub mod utils;

#[derive(Debug, Clone, Copy, ToString, PartialEq, Eq)]
#[strum(serialize_all = "lowercase")]
pub enum CustomType {
    Struct,
    Enum,
}

/// Fuel ABI representation in JSON, originally specified here:
///
/// https://github.com/FuelLabs/fuel-specs/blob/master/specs/protocol/abi.md
///
/// This type may be used by compilers (e.g. Sway) and related tooling to convert an ABI
/// representation into native Rust structs and vice-versa.
#[derive(Default, Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgramABI {
    pub types: Vec<TypeDeclaration>,
    pub functions: Vec<ABIFunction>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ABIFunction {
    pub inputs: Vec<TypeApplication>,
    pub name: String,
    pub output: TypeApplication,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TypeDeclaration {
    pub type_id: usize,
    #[serde(rename = "type")]
    pub type_field: String,
    pub components: Option<Vec<TypeApplication>>, // Used for custom types
    pub type_parameters: Option<Vec<usize>>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TypeApplication {
    pub name: String,
    #[serde(rename = "type")]
    pub type_id: usize,
    pub type_arguments: Option<Vec<TypeApplication>>,
}

impl TypeDeclaration {
    pub fn is_enum_type(&self) -> bool {
        const ENUM_KEYWORD: &str = "enum ";
        self.type_field.starts_with(ENUM_KEYWORD)
    }

    pub fn is_struct_type(&self) -> bool {
        const STRUCT_KEYWORD: &str = "struct ";
        self.type_field.starts_with(STRUCT_KEYWORD)
    }

    pub fn is_option(&self) -> bool {
        const OPTION_KEYWORD: &str = " Option";
        self.type_field.ends_with(OPTION_KEYWORD)
    }

    pub fn is_result(&self) -> bool {
        const RESULT_KEYWORD: &str = " Result";
        self.type_field.ends_with(RESULT_KEYWORD)
    }
}
