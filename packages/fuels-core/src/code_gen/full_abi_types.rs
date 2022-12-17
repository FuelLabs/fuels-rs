use fuels_types::errors::Error;
use fuels_types::{ABIFunction, LoggedType, ProgramABI, TypeApplication, TypeDeclaration};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub(crate) struct FullProgramABI {
    pub types: Vec<FullTypeDeclaration>,
    pub functions: Vec<FullABIFunction>,
    pub logged_types: Vec<FullLoggedType>,
}

impl FullProgramABI {
    pub fn from_json_abi(abi: &str) -> Result<Self, Error> {
        let parsed_abi: ProgramABI = serde_json::from_str(abi)?;
        Ok(FullProgramABI::from_counterpart(&parsed_abi))
    }

    fn from_counterpart(program_abi: &ProgramABI) -> FullProgramABI {
        let lookup: HashMap<_, _> = program_abi
            .types
            .iter()
            .map(|ttype| (ttype.type_id, ttype.clone()))
            .collect();

        let types = program_abi
            .types
            .iter()
            .map(|ttype| FullTypeDeclaration::from_counterpart(ttype, &lookup))
            .collect();

        let functions = program_abi
            .functions
            .iter()
            .map(|fun| FullABIFunction::from_counterpart(fun, &lookup))
            .collect();

        let logged_types = program_abi
            .logged_types
            .iter()
            .flatten()
            .map(|logged_type| FullLoggedType::from_counterpart(logged_type, &lookup))
            .collect();

        Self {
            types,
            functions,
            logged_types,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct FullABIFunction {
    pub inputs: Vec<FullTypeApplication>,
    pub name: String,
    pub output: FullTypeApplication,
}

impl FullABIFunction {
    pub(crate) fn from_counterpart(
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

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) struct FullTypeDeclaration {
    pub type_field: String,
    pub components: Vec<FullTypeApplication>,
    pub type_parameters: Vec<FullTypeDeclaration>,
}

impl FullTypeDeclaration {
    pub(crate) fn from_counterpart(
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

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) struct FullTypeApplication {
    pub name: String,
    pub type_decl: FullTypeDeclaration,
    pub type_arguments: Vec<FullTypeApplication>,
}

impl FullTypeApplication {
    pub(crate) fn from_counterpart(
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
pub(crate) struct FullLoggedType {
    pub log_id: u64,
    pub application: FullTypeApplication,
}

impl FullLoggedType {
    fn from_counterpart(
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
