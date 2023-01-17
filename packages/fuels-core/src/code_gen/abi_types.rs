use std::collections::HashMap;

use fuel_abi_types::program_abi::{
    ABIFunction, LoggedType, ProgramABI, TypeApplication, TypeDeclaration,
};
use fuels_types::errors::Error;
use fuels_types::errors::Error::InvalidData;

/// 'Full' versions of the ABI structures are needed to simplify duplicate
/// detection later on. The original ones([`ProgramABI`], [`TypeApplication`],
/// [`TypeDeclaration`] and others) are not suited for this due to their use of
/// ids, which might differ between contracts even though the type they
/// represent is virtually the same.
#[derive(Debug, Clone)]
pub(crate) struct FullProgramABI {
    pub types: Vec<FullTypeDeclaration>,
    pub functions: Vec<FullABIFunction>,
    pub logged_types: Vec<FullLoggedType>,
}

impl FullProgramABI {
    pub fn from_json_abi(abi: &str) -> Result<Self, Error> {
        let parsed_abi: ProgramABI = serde_json::from_str(abi)?;
        FullProgramABI::from_counterpart(&parsed_abi)
    }

    fn from_counterpart(program_abi: &ProgramABI) -> Result<FullProgramABI, Error> {
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
            .collect::<Result<Vec<_>, _>>()?;

        let logged_types = program_abi
            .logged_types
            .iter()
            .flatten()
            .map(|logged_type| FullLoggedType::from_counterpart(logged_type, &lookup))
            .collect();

        Ok(Self {
            types,
            functions,
            logged_types,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FullABIFunction {
    name: String,
    inputs: Vec<FullTypeApplication>,
    output: FullTypeApplication,
}

impl FullABIFunction {
    pub(crate) fn new(
        name: String,
        inputs: Vec<FullTypeApplication>,
        output: FullTypeApplication,
    ) -> Result<Self, Error> {
        if name.is_empty() {
            Err(InvalidData(
                "FullABIFunction's name cannot be empty!".to_string(),
            ))
        } else {
            Ok(Self {
                name,
                inputs,
                output,
            })
        }
    }

    pub(crate) fn name(&self) -> &str {
        self.name.as_str()
    }

    pub(crate) fn inputs(&self) -> &[FullTypeApplication] {
        self.inputs.as_slice()
    }

    pub(crate) fn output(&self) -> &FullTypeApplication {
        &self.output
    }

    pub(crate) fn from_counterpart(
        abi_function: &ABIFunction,
        types: &HashMap<usize, TypeDeclaration>,
    ) -> Result<FullABIFunction, Error> {
        let inputs = abi_function
            .inputs
            .iter()
            .map(|input| FullTypeApplication::from_counterpart(input, types))
            .collect();

        FullABIFunction::new(
            abi_function.name.clone(),
            inputs,
            FullTypeApplication::from_counterpart(&abi_function.output, types),
        )
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn abi_function_cannot_have_an_empty_name() {
        let fn_output = FullTypeApplication {
            name: "".to_string(),
            type_decl: FullTypeDeclaration {
                type_field: "SomeType".to_string(),
                components: vec![],
                type_parameters: vec![],
            },
            type_arguments: vec![],
        };

        let err = FullABIFunction::new("".to_string(), vec![], fn_output)
            .expect_err("Should have failed.");

        if let InvalidData(msg) = err {
            assert_eq!(msg, "FullABIFunction's name cannot be empty!");
        } else {
            panic!("Unexpected error: {err}");
        }
    }
    #[test]
    fn can_convert_into_full_type_decl() {
        // given
        let type_0 = TypeDeclaration {
            type_id: 0,
            type_field: "type_0".to_string(),
            components: Some(vec![TypeApplication {
                name: "type_0_component_a".to_string(),
                type_id: 1,
                type_arguments: Some(vec![TypeApplication {
                    name: "type_0_type_arg_0".to_string(),
                    type_id: 2,
                    type_arguments: None,
                }]),
            }]),
            type_parameters: Some(vec![2]),
        };

        let type_1 = TypeDeclaration {
            type_id: 1,
            type_field: "type_1".to_string(),
            components: None,
            type_parameters: None,
        };

        let type_2 = TypeDeclaration {
            type_id: 2,
            type_field: "type_2".to_string(),
            components: None,
            type_parameters: None,
        };

        let types = [&type_0, &type_1, &type_2]
            .iter()
            .map(|&ttype| (ttype.type_id, ttype.clone()))
            .collect::<HashMap<_, _>>();

        // when
        let sut = FullTypeDeclaration::from_counterpart(&type_0, &types);

        // then
        let type_2_decl = FullTypeDeclaration {
            type_field: "type_2".to_string(),
            components: vec![],
            type_parameters: vec![],
        };
        assert_eq!(
            sut,
            FullTypeDeclaration {
                type_field: "type_0".to_string(),
                components: vec![FullTypeApplication {
                    name: "type_0_component_a".to_string(),
                    type_decl: FullTypeDeclaration {
                        type_field: "type_1".to_string(),
                        components: vec![],
                        type_parameters: vec![],
                    },
                    type_arguments: vec![FullTypeApplication {
                        name: "type_0_type_arg_0".to_string(),
                        type_decl: type_2_decl.clone(),
                        type_arguments: vec![],
                    },],
                },],
                type_parameters: vec![type_2_decl],
            }
        )
    }

    #[test]
    fn can_convert_into_full_type_appl() {
        let application = TypeApplication {
            name: "ta_0".to_string(),
            type_id: 0,
            type_arguments: Some(vec![TypeApplication {
                name: "ta_1".to_string(),
                type_id: 1,
                type_arguments: None,
            }]),
        };

        let type_0 = TypeDeclaration {
            type_id: 0,
            type_field: "type_0".to_string(),
            components: None,
            type_parameters: None,
        };

        let type_1 = TypeDeclaration {
            type_id: 1,
            type_field: "type_1".to_string(),
            components: None,
            type_parameters: None,
        };

        let types = [&type_0, &type_1]
            .into_iter()
            .map(|ttype| (ttype.type_id, ttype.clone()))
            .collect::<HashMap<_, _>>();

        // given
        let sut = FullTypeApplication::from_counterpart(&application, &types);

        // then
        assert_eq!(
            sut,
            FullTypeApplication {
                name: "ta_0".to_string(),
                type_decl: FullTypeDeclaration {
                    type_field: "type_0".to_string(),
                    components: vec![],
                    type_parameters: vec![],
                },
                type_arguments: vec![FullTypeApplication {
                    name: "ta_1".to_string(),
                    type_decl: FullTypeDeclaration {
                        type_field: "type_1".to_string(),
                        components: vec![],
                        type_parameters: vec![],
                    },
                    type_arguments: vec![],
                },],
            }
        )
    }
}
