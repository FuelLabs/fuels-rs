//! Defines a set of serializable types required for the Fuel VM ABI.
//!
//! We declare these in a dedicated, minimal crate in order to allow for downstream projects to
//! consume or generate these ABI-compatible types without needing to pull in the rest of the SDK.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use strum_macros::ToString;
use utils::{has_array_format, has_tuple_format};

pub mod bech32;
pub mod constants;
pub mod errors;
pub mod function_selector;
pub mod param_types;
pub mod parse_param;
pub mod utils;

// Both those constants are used to determine if a type field represents an `Enum` or a `Struct`.
// Since it would have the format `struct foo` or `enum bar`, there is a whitespace.
pub const STRUCT_KEYWORD: &str = "struct ";
pub const ENUM_KEYWORD: &str = "enum ";

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
    pub type_field: usize,
    pub type_arguments: Option<Vec<usize>>,
}

impl TypeDeclaration {
    pub fn is_enum_type(&self) -> bool {
        self.type_field.starts_with(ENUM_KEYWORD)
    }
    pub fn is_struct_type(&self) -> bool {
        self.type_field.starts_with(STRUCT_KEYWORD)
    }
    pub fn is_custom_type(&self, types: &HashMap<usize, TypeDeclaration>) -> bool {
        self.is_enum_type()
            || self.is_struct_type()
            || self.has_custom_type_in_array(types)
            || self.has_custom_type_in_tuple(types)
    }

    pub fn has_custom_type_in_array(&self, types: &HashMap<usize, TypeDeclaration>) -> bool {
        if has_array_format(&self.type_field) {
            // For each component in the tuple, check if it is a custom type
            for component in self.components.as_ref().unwrap() {
                let component_type = types.get(&component.type_field).unwrap();
                if component_type.is_custom_type(types) {
                    return true;
                }
            }

            return self.get_custom_type().is_some();
        }
        false
    }

    pub fn has_custom_type_in_tuple(&self, types: &HashMap<usize, TypeDeclaration>) -> bool {
        if has_tuple_format(&self.type_field) {
            // For each component in the tuple, check if it is a custom type
            for component in self.components.as_ref().unwrap() {
                let component_type = types.get(&component.type_field).unwrap();
                if component_type.is_custom_type(types) {
                    return true;
                }
            }
        }
        false
    }

    pub fn get_custom_type(&self) -> Option<CustomType> {
        if self.type_field.contains(STRUCT_KEYWORD) {
            Some(CustomType::Struct)
        } else if self.type_field.contains(ENUM_KEYWORD) {
            Some(CustomType::Enum)
        } else {
            None
        }
    }
}
