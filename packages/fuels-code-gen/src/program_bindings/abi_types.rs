use std::collections::HashMap;

use fuel_abi_types::{
    program_abi::{
        ABIFunction, Attribute, Configurable, LoggedType, ProgramABI, TypeApplication,
        TypeDeclaration,
    },
    utils::extract_custom_type_name,
};

use crate::{
    error::{error, Result},
    utils::TypePath,
};

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
    pub configurables: Vec<FullConfigurable>,
}

impl FullProgramABI {
    pub fn from_json_abi(abi: &str) -> Result<Self> {
        let parsed_abi: ProgramABI = serde_json::from_str(abi)?;
        FullProgramABI::from_counterpart(&parsed_abi)
    }

    fn from_counterpart(program_abi: &ProgramABI) -> Result<FullProgramABI> {
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
            .collect::<Result<Vec<_>>>()?;

        let logged_types = program_abi
            .logged_types
            .iter()
            .flatten()
            .map(|logged_type| FullLoggedType::from_counterpart(logged_type, &lookup))
            .collect();

        let configurables = program_abi
            .configurables
            .iter()
            .flatten()
            .map(|configurable| FullConfigurable::from_counterpart(configurable, &lookup))
            .collect();

        Ok(Self {
            types,
            functions,
            logged_types,
            configurables,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FullABIFunction {
    name: String,
    inputs: Vec<FullTypeApplication>,
    output: FullTypeApplication,
    attributes: Vec<Attribute>,
}

impl FullABIFunction {
    pub(crate) fn new(
        name: String,
        inputs: Vec<FullTypeApplication>,
        output: FullTypeApplication,
        attributes: Vec<Attribute>,
    ) -> Result<Self> {
        if name.is_empty() {
            Err(error!("FullABIFunction's name cannot be empty!"))
        } else {
            Ok(Self {
                name,
                inputs,
                output,
                attributes,
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

    pub(crate) fn is_payable(&self) -> bool {
        self.attributes.iter().any(|attr| attr.name == "payable")
    }

    pub(crate) fn from_counterpart(
        abi_function: &ABIFunction,
        types: &HashMap<usize, TypeDeclaration>,
    ) -> Result<FullABIFunction> {
        let inputs = abi_function
            .inputs
            .iter()
            .map(|input| FullTypeApplication::from_counterpart(input, types))
            .collect();

        let attributes = abi_function
            .attributes
            .as_ref()
            .map_or(vec![], Clone::clone);
        FullABIFunction::new(
            abi_function.name.clone(),
            inputs,
            FullTypeApplication::from_counterpart(&abi_function.output, types),
            attributes,
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

    pub(crate) fn custom_type_path(&self) -> Result<TypePath> {
        let type_field = &self.type_field;
        let type_name = extract_custom_type_name(type_field)
            .ok_or_else(|| error!("Couldn't extract custom type path from '{type_field}'"))?;

        TypePath::new(type_name)
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

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) struct FullConfigurable {
    pub name: String,
    pub application: FullTypeApplication,
    pub offset: u64,
}

impl FullConfigurable {
    pub(crate) fn from_counterpart(
        configurable: &Configurable,
        types: &HashMap<usize, TypeDeclaration>,
    ) -> FullConfigurable {
        FullConfigurable {
            name: configurable.name.clone(),
            application: FullTypeApplication::from_counterpart(&configurable.application, types),
            offset: configurable.offset,
        }
    }
}

impl FullTypeDeclaration {
    pub fn is_custom_type(&self) -> bool {
        self.is_struct_type() || self.is_enum_type()
    }

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
    use std::collections::HashMap;

    use super::*;

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

        let err = FullABIFunction::new("".to_string(), vec![], fn_output, vec![])
            .expect_err("Should have failed.");

        assert_eq!(err.to_string(), "FullABIFunction's name cannot be empty!");
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
