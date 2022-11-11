//! Defines a set of serializable types required for the Fuel VM ABI.
//!
//! We declare these in a dedicated, minimal crate in order to allow for downstream projects to
//! consume or generate these ABI-compatible types without needing to pull in the rest of the SDK.

use itertools::chain;
use proc_macro2::TokenStream;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use strum_macros::ToString;

pub mod bech32;
pub mod constants;
pub mod enum_variants;
pub mod errors;
pub mod param_types;
pub mod utils;

#[derive(Debug, Clone, Copy, ToString, PartialEq, Eq)]
#[strum(serialize_all = "lowercase")]
pub enum CustomType {
    Struct,
    Enum,
}

/// FuelVM ABI representation in JSON, originally specified here:
///
/// https://github.com/FuelLabs/fuel-specs/blob/master/specs/protocol/abi.md
///
/// This type may be used by compilers and related tooling to convert an ABI
/// representation into native Rust structs and vice-versa.
#[derive(Default, Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgramABI {
    pub types: Vec<TypeDeclaration>,
    pub functions: Vec<ABIFunction>,
    pub logged_types: Option<Vec<LoggedType>>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ABIFunction {
    pub inputs: Vec<TypeApplication>,
    pub name: String,
    pub output: TypeApplication,
}

impl ABIFunction {
    pub fn to_full_function(&self, types: &HashMap<usize, TypeDeclaration>) -> FullABIFunction {
        let inputs = self
            .inputs
            .iter()
            .map(|input| input.to_full_application(types))
            .collect();
        FullABIFunction {
            inputs,
            name: self.name.clone(),
            output: self.output.to_full_application(types),
        }
    }
}

#[derive(Debug, Clone)]
pub struct FullABIFunction {
    pub inputs: Vec<FullTypeApplication>,
    pub name: String,
    pub output: FullTypeApplication,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FullTypeDeclaration {
    pub type_field: String,
    pub components: Vec<FullTypeApplication>,
    pub type_parameters: Vec<FullTypeDeclaration>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TypeApplication {
    pub name: String,
    #[serde(rename = "type")]
    pub type_id: usize,
    pub type_arguments: Option<Vec<TypeApplication>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FullTypeApplication {
    pub name: String,
    pub type_decl: FullTypeDeclaration,
    pub type_arguments: Vec<FullTypeApplication>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoggedType {
    pub log_id: u64,
    #[serde(rename = "loggedType")]
    pub application: TypeApplication,
}
impl LoggedType {
    pub fn to_full_logged_type(&self, types: &HashMap<usize, TypeDeclaration>) -> FullLoggedType {
        FullLoggedType {
            log_id: self.log_id,
            application: self.application.to_full_application(types),
        }
    }
}

#[derive(Debug, Clone)]
pub struct FullLoggedType {
    pub log_id: u64,
    pub application: FullTypeApplication,
}

#[derive(Debug, Clone)]
pub struct ResolvedLog {
    pub log_id: u64,
    pub param_type_call: TokenStream,
    pub resolved_type_name: TokenStream,
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

impl TypeDeclaration {
    pub fn to_full_declaration(
        &self,
        types: &HashMap<usize, TypeDeclaration>,
    ) -> FullTypeDeclaration {
        let components = self
            .components
            .clone()
            .unwrap_or_default()
            .into_iter()
            .map(|application| application.to_full_application(types))
            .collect();
        let type_parameters = self
            .type_parameters
            .clone()
            .unwrap_or_default()
            .into_iter()
            .map(|id| types.get(&id).unwrap().to_full_declaration(types))
            .collect();
        FullTypeDeclaration {
            type_field: self.type_field.clone(),
            components,
            type_parameters,
        }
    }
}
impl TypeApplication {
    pub fn to_full_application(
        &self,
        types: &HashMap<usize, TypeDeclaration>,
    ) -> FullTypeApplication {
        let type_arguments = self
            .type_arguments
            .clone()
            .unwrap_or_default()
            .into_iter()
            .map(|application| application.to_full_application(types))
            .collect();

        let type_decl = types
            .get(&self.type_id)
            .unwrap()
            .clone()
            .to_full_declaration(types);

        FullTypeApplication {
            name: self.name.clone(),
            type_decl,
            type_arguments,
        }
    }
}
