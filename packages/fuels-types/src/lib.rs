//! Defines a set of serializable types required for the Fuel VM ABI.
//!
//! We declare these in a dedicated, minimal crate in order to allow for downstream projects to
//! consume or generate these ABI-compatible types without needing to pull in the rest of the SDK.

use proc_macro2::TokenStream;
use serde::{Deserialize, Serialize};
use strum_macros::ToString;

pub mod bech32;
pub mod block;
pub mod chain_info;
pub mod coin;
pub mod constants;
pub mod enum_variants;
pub mod errors;
pub mod message;
pub mod message_proof;
pub mod node_info;
pub mod param_types;
pub mod resource;
pub mod transaction_response;
pub mod utils;

#[derive(Debug, Clone, Copy, ToString, PartialEq, Eq)]
#[strum(serialize_all = "lowercase")]
pub enum CustomType {
    Struct,
    Enum,
}

/// FuelVM ABI representation in JSON, originally specified
/// [here](https://github.com/FuelLabs/fuel-specs/blob/master/specs/protocol/abi.md).
///
/// This type may be used by compilers and related tooling to convert an ABI
/// representation into native Rust structs and vice-versa.
#[derive(Default, Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgramABI {
    pub types: Vec<TypeDeclaration>,
    pub functions: Vec<ABIFunction>,
    pub logged_types: Option<Vec<LoggedType>>,
    pub messages_types: Option<Vec<MessageType>>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ABIFunction {
    pub inputs: Vec<TypeApplication>,
    pub name: String,
    pub output: TypeApplication,
    pub attributes: Option<Vec<Attribute>>,
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

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoggedType {
    pub log_id: u64,
    #[serde(rename = "loggedType")]
    pub application: TypeApplication,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageType {
    pub message_id: u64,
    #[serde(rename = "messageType")]
    pub application: TypeApplication,
}

#[derive(Debug, Clone)]
pub struct ResolvedLog {
    pub log_id: u64,
    pub param_type_call: TokenStream,
    pub resolved_type_name: TokenStream,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Attribute {
    pub name: String,
    pub arguments: Vec<String>,
}

impl TypeDeclaration {
    pub fn is_enum_type(&self) -> bool {
        self.type_field.starts_with("enum ")
    }

    pub fn is_struct_type(&self) -> bool {
        self.type_field.starts_with("struct ")
    }
}
