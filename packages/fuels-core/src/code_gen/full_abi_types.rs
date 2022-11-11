use fuels_types::{ABIFunction, LoggedType, TypeApplication, TypeDeclaration};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct FullABIFunction {
    pub inputs: Vec<FullTypeApplication>,
    pub name: String,
    pub output: FullTypeApplication,
}

impl FullABIFunction {
    pub fn from_counterpart(
        abi_function: &ABIFunction,
        types: &HashMap<usize, TypeDeclaration>,
    ) -> FullABIFunction {
        let inputs = abi_function
            .inputs
            .iter()
            .map(|input| FullTypeApplication::from_counterpart(input, types))
            .collect();
        FullABIFunction {
            inputs,
            name: abi_function.name.clone(),
            output: FullTypeApplication::from_counterpart(&abi_function.output, types),
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
    pub fn from_counterpart(
        type_decl: &TypeDeclaration,
        types: &HashMap<usize, TypeDeclaration>,
    ) -> FullTypeDeclaration {
        let components = type_decl
            .components
            .clone()
            .unwrap_or_default()
            .into_iter()
            .map(|application| FullTypeApplication::from_counterpart(&application, types))
            .collect();
        let type_parameters = type_decl
            .type_parameters
            .clone()
            .unwrap_or_default()
            .into_iter()
            .map(|id| FullTypeDeclaration::from_counterpart(types.get(&id).unwrap(), types))
            .collect();
        FullTypeDeclaration {
            type_field: type_decl.type_field.clone(),
            components,
            type_parameters,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FullTypeApplication {
    pub name: String,
    pub type_decl: FullTypeDeclaration,
    pub type_arguments: Vec<FullTypeApplication>,
}

impl FullTypeApplication {
    pub fn from_counterpart(
        type_application: &TypeApplication,
        types: &HashMap<usize, TypeDeclaration>,
    ) -> FullTypeApplication {
        let type_arguments = type_application
            .type_arguments
            .clone()
            .unwrap_or_default()
            .into_iter()
            .map(|application| FullTypeApplication::from_counterpart(&application, types))
            .collect();

        let type_decl = FullTypeDeclaration::from_counterpart(
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
            application: FullTypeApplication::from_counterpart(&logged_type.application, types),
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
