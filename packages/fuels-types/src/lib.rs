//! Defines a set of serializable types required for the Fuel VM ABI.
//!
//! We declare these in a dedicated, minimal crate in order to allow for downstream projects to
//! consume or generate these ABI-compatible types without needing to pull in the rest of the SDK.

use serde::{Deserialize, Serialize};
use strum_macros::ToString;

/// Fuel ABI representation in JSON, originally specified here:
///
/// https://github.com/FuelLabs/fuel-specs/blob/master/specs/protocol/abi.md
///
/// This type may be used by compilers (e.g. Sway) and related tooling to convert an ABI
/// representation into native Rust structs and vice-versa.
pub type JsonABI = Vec<Function>;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Function {
    #[serde(rename = "type")]
    pub type_field: String,
    pub inputs: Vec<Property>,
    pub name: String,
    pub outputs: Vec<Property>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Property {
    pub name: String,
    #[serde(rename = "type")]
    pub type_field: String,
    pub components: Option<Vec<Property>>, // Used for custom types
}

// Both those constants are used to determine if a type field represents an `Enum` or a `Struct`.
// Since it would have the format `struct foo` or `enum bar`, there is a whitespace.
const STRUCT_KEYWORD: &str = "struct ";
const ENUM_KEYWORD: &str = "enum ";

impl Property {
    pub fn is_enum_type(&self) -> bool {
        self.type_field.starts_with(ENUM_KEYWORD)
    }
    pub fn is_struct_type(&self) -> bool {
        self.type_field.starts_with(STRUCT_KEYWORD)
    }
    pub fn is_custom_type(&self) -> bool {
        self.is_enum_type() || self.is_struct_type()
    }
}

#[derive(Debug, Clone, ToString, PartialEq, Eq)]
#[strum(serialize_all = "lowercase")]
pub enum CustomType {
    Struct,
    Enum,
}
