use fuels_types::param_types::ParamType;
use fuels_types::{ABIFunction, TypeApplication, TypeDeclaration};
use itertools::Itertools;
use std::collections::HashMap;
use std::iter::zip;

/// Given a `ABIFunction` will return a String representing the function
/// selector as specified in the Fuel specs.
pub fn resolve_fn_selector(
    function: &ABIFunction,
    types: &HashMap<usize, TypeDeclaration>,
) -> String {
    let fun_args = function
        .inputs
        .iter()
        .map(|input| resolve_function_arg(input, types))
        .collect::<Vec<_>>()
        .join(",");

    format!("{}({})", function.name, fun_args)
}

#[derive(Debug, Clone)]
struct Type {
    param_type: ParamType,
    generic_params: Vec<Type>,
    components: Vec<Type>,
}

/// Will recursively drill down the given generic parameters until all types are
/// resolved.
///
/// # Arguments
///
/// * `type_application`: the type we wish to resolve
/// * `types`: all types used in the function call
/// * `parent_generic_params`: a slice of generic_type_id -> Type mapping indicating
///    to what type a generic parameter should resolve to.
fn resolve_type_application(
    type_application: &TypeApplication,
    types: &HashMap<usize, TypeDeclaration>,
    parent_generic_params: &[(usize, Type)],
) -> Type {
    let type_decl = types.get(&type_application.type_id).unwrap();
    let param_type = ParamType::from_type_declaration(type_decl, types).unwrap();

    if let ParamType::Generic(_) = &param_type {
        let (_, generic_type) = parent_generic_params
            .iter()
            .find(|(id, _)| *id == type_application.type_id)
            .unwrap();

        return generic_type.clone();
    }

    // Figure out what does the current type do with the inherited generic
    // parameters and reestablish the mapping since the current type might have
    // renamed the inherited generic parameters.
    let generic_params_lookup =
        determine_generics_for_type(type_application, types, parent_generic_params);

    // Resolve the enclosed components (if any) with the newly resolved generic
    // parameters.
    let components = type_decl
        .components
        .iter()
        .flatten()
        .map(|component| resolve_type_application(component, types, &generic_params_lookup))
        .collect_vec();

    Type {
        param_type,
        components,
        generic_params: generic_params_lookup
            .into_iter()
            .map(|(_, ty)| ty)
            .collect(),
    }
}

/// For the given type generates generic_type_id -> Type mapping describing to
/// which types generic parameters should be resolved.
///
/// # Arguments
///
/// * `type_application`: The type on which the generic parameters are defined.
/// * `types`: All types used.
/// * `parent_generic_params`: The generic parameters as inherited from the
///                            enclosing type (a struct/enum/array etc.).
fn determine_generics_for_type(
    type_application: &TypeApplication,
    types: &HashMap<usize, TypeDeclaration>,
    parent_generic_params: &[(usize, Type)],
) -> Vec<(usize, Type)> {
    let type_decl = types.get(&type_application.type_id).unwrap();
    match &type_decl.type_parameters {
        // The presence of type_parameters indicates that the current type
        // (a struct or an enum) defines some generic parameters (i.e.
        // SomeStruct<T, K>
        Some(params) if !params.is_empty() => {
            // Determine what Types the generics will resolve to.
            let generic_params_from_current_type = type_application
                .type_arguments
                .iter()
                .flatten()
                .map(|ty| resolve_type_application(ty, types, parent_generic_params))
                .collect_vec();

            let generics_to_use = if !generic_params_from_current_type.is_empty() {
                generic_params_from_current_type
            } else {
                // Types such as arrays and enums inherit and forward their
                // generic parameters, without declaring their own.
                parent_generic_params
                    .iter()
                    .map(|(_, ty)| ty)
                    .cloned()
                    .collect()
            };

            // All inherited but unused generic types are dropped. The rest are
            // re-mapped to new type_ids since child types are free to rename
            // the generic parameters as they see fit -- i.e.
            // struct ParentStruct<T>{
            //     b: ChildStruct<T>
            // }
            // struct ChildStruct<K> {
            //     c: K
            // }

            zip(params.clone(), generics_to_use).collect()
        }
        _ => parent_generic_params.to_vec(),
    }
}

impl Type {
    pub fn to_fn_selector_format(&self) -> String {
        let get_components = || {
            self.components
                .iter()
                .map(|component| component.to_fn_selector_format())
                .collect::<Vec<_>>()
                .join(",")
        };

        let get_generics = || {
            let generics = self
                .generic_params
                .iter()
                .map(|arg| arg.to_fn_selector_format())
                .collect::<Vec<_>>()
                .join(",");

            if generics.is_empty() {
                String::new()
            } else {
                format!("<{generics}>")
            }
        };

        match &self.param_type {
            ParamType::U8 => "u8".to_owned(),
            ParamType::U16 => "u16".to_owned(),
            ParamType::U32 => "u32".to_owned(),
            ParamType::U64 => "u64".to_owned(),
            ParamType::Bool => "bool".to_owned(),
            ParamType::Byte => "byte".to_owned(),
            ParamType::B256 => "b256".to_owned(),
            ParamType::Unit => "()".to_owned(),
            ParamType::String(len) => {
                format!("str[{len}]")
            }
            ParamType::Array(_, len) => {
                let components = get_components();
                format!("a[{components};{len}]")
            }
            ParamType::Struct(_) => {
                let generics = get_generics();
                let components = get_components();
                format!("s{generics}({components})")
            }
            ParamType::Enum(_) => {
                let generics = get_generics();
                let components = get_components();
                format!("e{generics}({components})")
            }
            ParamType::Tuple(_) => {
                let components = get_components();
                format!("({components})")
            }
            ParamType::Generic(_name) => {
                panic!("ParamType::Generic cannot appear in a function selector!")
            }
        }
    }
}

fn resolve_function_arg(arg: &TypeApplication, types: &HashMap<usize, TypeDeclaration>) -> String {
    let resolved_type = resolve_type_application(arg, types, Default::default());

    resolved_type.to_fn_selector_format()
}

#[cfg(test)]
mod tests {
    use super::*;
    use fuels_types::{ABIFunction, ProgramABI};

    #[test]
    fn handles_primitive_types() {
        let check_selector_for_type = |primitive_type: &str| {
            let fun = ABIFunction {
                inputs: vec![TypeApplication {
                    name: "arg".to_string(),
                    type_id: 0,
                    type_arguments: None,
                }],
                name: "some_fun".to_string(),
                output: Default::default(),
            };

            let types = [TypeDeclaration {
                type_id: 0,
                type_field: primitive_type.to_string(),
                components: None,
                type_parameters: None,
            }]
            .map(|decl| (decl.type_id, decl))
            .into();

            let selector = resolve_fn_selector(&fun, &types);

            assert_eq!(selector, format!("some_fun({})", primitive_type));
        };

        for primitive_type in [
            "u8", "u16", "u32", "u64", "bool", "byte", "b256", "()", "str[15]",
        ] {
            check_selector_for_type(primitive_type);
        }
    }

    #[test]
    fn handles_arrays() {
        let fun = ABIFunction {
            inputs: vec![TypeApplication {
                name: "arg".to_string(),
                type_id: 0,
                type_arguments: None,
            }],
            name: "some_fun".to_string(),
            output: Default::default(),
        };

        let types = [
            TypeDeclaration {
                type_id: 0,
                type_field: "[_; 1]".to_string(),
                components: Some(vec![TypeApplication {
                    name: "__array_element".to_string(),
                    type_id: 1,
                    type_arguments: None,
                }]),
                type_parameters: None,
            },
            TypeDeclaration {
                type_id: 1,
                type_field: "u8".to_string(),
                components: None,
                type_parameters: None,
            },
        ]
        .map(|decl| (decl.type_id, decl))
        .into();

        let selector = resolve_fn_selector(&fun, &types);

        assert_eq!(selector, format!("some_fun(a[u8;1])"));
    }

    #[test]
    fn handles_tuples() {
        let fun = ABIFunction {
            inputs: vec![TypeApplication {
                name: "arg".to_string(),
                type_id: 0,
                type_arguments: None,
            }],
            name: "some_fun".to_string(),
            output: Default::default(),
        };

        let types = [
            TypeDeclaration {
                type_id: 0,
                type_field: "(_, _)".to_string(),
                components: Some(vec![
                    TypeApplication {
                        name: "__tuple_element".to_string(),
                        type_id: 1,
                        type_arguments: None,
                    },
                    TypeApplication {
                        name: "__tuple_element".to_string(),
                        type_id: 1,
                        type_arguments: None,
                    },
                ]),
                type_parameters: None,
            },
            TypeDeclaration {
                type_id: 1,
                type_field: "u8".to_string(),
                components: None,
                type_parameters: None,
            },
        ]
        .map(|decl| (decl.type_id, decl))
        .into();

        let selector = resolve_fn_selector(&fun, &types);

        assert_eq!(selector, format!("some_fun((u8,u8))"));
    }

    #[test]
    fn handles_structs() {
        let fun = ABIFunction {
            inputs: vec![TypeApplication {
                name: "arg".to_string(),
                type_id: 2,
                type_arguments: Some(vec![TypeApplication {
                    name: "".to_string(),
                    type_id: 3,
                    type_arguments: None,
                }]),
            }],
            name: "some_fun".to_string(),
            output: Default::default(),
        };

        let types = [
            TypeDeclaration {
                type_id: 1,
                type_field: "generic T".to_string(),
                components: None,
                type_parameters: None,
            },
            TypeDeclaration {
                type_id: 2,
                type_field: "struct SomeStruct".to_string(),
                components: Some(vec![
                    TypeApplication {
                        name: "a".to_string(),
                        type_id: 4,
                        type_arguments: None,
                    },
                    TypeApplication {
                        name: "b".to_string(),
                        type_id: 1,
                        type_arguments: None,
                    },
                ]),
                type_parameters: Some(vec![1]),
            },
            TypeDeclaration {
                type_id: 3,
                type_field: "u32".to_string(),
                components: None,
                type_parameters: None,
            },
            TypeDeclaration {
                type_id: 4,
                type_field: "u64".to_string(),
                components: None,
                type_parameters: None,
            },
        ]
        .map(|decl| (decl.type_id, decl))
        .into();

        let selector = resolve_fn_selector(&fun, &types);

        assert_eq!(selector, format!("some_fun(s<u32>(u64,u32))"));
    }

    #[test]
    fn handles_enums() {
        let fun = ABIFunction {
            inputs: vec![TypeApplication {
                name: "arg".to_string(),
                type_id: 2,
                type_arguments: Some(vec![TypeApplication {
                    name: "".to_string(),
                    type_id: 3,
                    type_arguments: None,
                }]),
            }],
            name: "some_fun".to_string(),
            output: Default::default(),
        };

        let types = [
            TypeDeclaration {
                type_id: 1,
                type_field: "generic T".to_string(),
                components: None,
                type_parameters: None,
            },
            TypeDeclaration {
                type_id: 2,
                type_field: "enum SomeEnum".to_string(),
                components: Some(vec![
                    TypeApplication {
                        name: "a".to_string(),
                        type_id: 4,
                        type_arguments: None,
                    },
                    TypeApplication {
                        name: "b".to_string(),
                        type_id: 1,
                        type_arguments: None,
                    },
                ]),
                type_parameters: Some(vec![1]),
            },
            TypeDeclaration {
                type_id: 3,
                type_field: "u32".to_string(),
                components: None,
                type_parameters: None,
            },
            TypeDeclaration {
                type_id: 4,
                type_field: "u64".to_string(),
                components: None,
                type_parameters: None,
            },
        ]
        .map(|decl| (decl.type_id, decl))
        .into();

        let selector = resolve_fn_selector(&fun, &types);

        assert_eq!(selector, format!("some_fun(e<u32>(u64,u32))"));
    }

    #[test]
    fn ultimate_test() {
        let abi = ProgramABI {
            types: vec![
                TypeDeclaration {
                    type_id: 0,
                    type_field: "()".to_string(),
                    components: Some(vec![]),
                    type_parameters: None,
                },
                TypeDeclaration {
                    type_id: 1,
                    type_field: "(_, _)".to_string(),
                    components: Some(vec![
                        TypeApplication {
                            name: "__tuple_element".to_string(),
                            type_id: 11,
                            type_arguments: None,
                        },
                        TypeApplication {
                            name: "__tuple_element".to_string(),
                            type_id: 11,
                            type_arguments: None,
                        },
                    ]),
                    type_parameters: None,
                },
                TypeDeclaration {
                    type_id: 2,
                    type_field: "(_, _)".to_string(),
                    components: Some(vec![
                        TypeApplication {
                            name: "__tuple_element".to_string(),
                            type_id: 5,
                            type_arguments: None,
                        },
                        TypeApplication {
                            name: "__tuple_element".to_string(),
                            type_id: 13,
                            type_arguments: None,
                        },
                    ]),
                    type_parameters: None,
                },
                TypeDeclaration {
                    type_id: 3,
                    type_field: "(_, _)".to_string(),
                    components: Some(vec![
                        TypeApplication {
                            name: "__tuple_element".to_string(),
                            type_id: 4,
                            type_arguments: None,
                        },
                        TypeApplication {
                            name: "__tuple_element".to_string(),
                            type_id: 21,
                            type_arguments: None,
                        },
                    ]),
                    type_parameters: None,
                },
                TypeDeclaration {
                    type_id: 4,
                    type_field: "[_; 1]".to_string(),
                    components: Some(vec![TypeApplication {
                        name: "__array_element".to_string(),
                        type_id: 8,
                        type_arguments: Some(vec![TypeApplication {
                            name: "".to_string(),
                            type_id: 20,
                            type_arguments: Some(vec![TypeApplication {
                                name: "".to_string(),
                                type_id: 19,
                                type_arguments: Some(vec![TypeApplication {
                                    name: "".to_string(),
                                    type_id: 17,
                                    type_arguments: Some(vec![TypeApplication {
                                        name: "".to_string(),
                                        type_id: 13,
                                        type_arguments: None,
                                    }]),
                                }]),
                            }]),
                        }]),
                    }]),
                    type_parameters: None,
                },
                TypeDeclaration {
                    type_id: 5,
                    type_field: "[_; 2]".to_string(),
                    components: Some(vec![TypeApplication {
                        name: "__array_element".to_string(),
                        type_id: 14,
                        type_arguments: None,
                    }]),
                    type_parameters: None,
                },
                TypeDeclaration {
                    type_id: 6,
                    type_field: "[_; 2]".to_string(),
                    components: Some(vec![TypeApplication {
                        name: "__array_element".to_string(),
                        type_id: 10,
                        type_arguments: None,
                    }]),
                    type_parameters: None,
                },
                TypeDeclaration {
                    type_id: 7,
                    type_field: "b256".to_string(),
                    components: None,
                    type_parameters: None,
                },
                TypeDeclaration {
                    type_id: 8,
                    type_field: "enum EnumWGeneric".to_string(),
                    components: Some(vec![
                        TypeApplication {
                            name: "a".to_string(),
                            type_id: 22,
                            type_arguments: None,
                        },
                        TypeApplication {
                            name: "b".to_string(),
                            type_id: 12,
                            type_arguments: None,
                        },
                    ]),
                    type_parameters: Some(vec![12]),
                },
                TypeDeclaration {
                    type_id: 9,
                    type_field: "generic K".to_string(),
                    components: None,
                    type_parameters: None,
                },
                TypeDeclaration {
                    type_id: 10,
                    type_field: "generic L".to_string(),
                    components: None,
                    type_parameters: None,
                },
                TypeDeclaration {
                    type_id: 11,
                    type_field: "generic M".to_string(),
                    components: None,
                    type_parameters: None,
                },
                TypeDeclaration {
                    type_id: 12,
                    type_field: "generic N".to_string(),
                    components: None,
                    type_parameters: None,
                },
                TypeDeclaration {
                    type_id: 13,
                    type_field: "generic T".to_string(),
                    components: None,
                    type_parameters: None,
                },
                TypeDeclaration {
                    type_id: 14,
                    type_field: "generic U".to_string(),
                    components: None,
                    type_parameters: None,
                },
                TypeDeclaration {
                    type_id: 15,
                    type_field: "str[2]".to_string(),
                    components: None,
                    type_parameters: None,
                },
                TypeDeclaration {
                    type_id: 16,
                    type_field: "struct MegaExample".to_string(),
                    components: Some(vec![
                        TypeApplication {
                            name: "a".to_string(),
                            type_id: 2,
                            type_arguments: None,
                        },
                        TypeApplication {
                            name: "b".to_string(),
                            type_id: 3,
                            type_arguments: None,
                        },
                    ]),
                    type_parameters: Some(vec![13, 14]),
                },
                TypeDeclaration {
                    type_id: 17,
                    type_field: "struct PassTheGenericOn".to_string(),
                    components: Some(vec![TypeApplication {
                        name: "one".to_string(),
                        type_id: 18,
                        type_arguments: Some(vec![TypeApplication {
                            name: "".to_string(),
                            type_id: 9,
                            type_arguments: None,
                        }]),
                    }]),
                    type_parameters: Some(vec![9]),
                },
                TypeDeclaration {
                    type_id: 18,
                    type_field: "struct SimpleGeneric".to_string(),
                    components: Some(vec![TypeApplication {
                        name: "single_generic_param".to_string(),
                        type_id: 13,
                        type_arguments: None,
                    }]),
                    type_parameters: Some(vec![13]),
                },
                TypeDeclaration {
                    type_id: 19,
                    type_field: "struct StructWArrayGeneric".to_string(),
                    components: Some(vec![TypeApplication {
                        name: "a".to_string(),
                        type_id: 6,
                        type_arguments: None,
                    }]),
                    type_parameters: Some(vec![10]),
                },
                TypeDeclaration {
                    type_id: 20,
                    type_field: "struct StructWTupleGeneric".to_string(),
                    components: Some(vec![TypeApplication {
                        name: "a".to_string(),
                        type_id: 1,
                        type_arguments: None,
                    }]),
                    type_parameters: Some(vec![11]),
                },
                TypeDeclaration {
                    type_id: 21,
                    type_field: "u32".to_string(),
                    components: None,
                    type_parameters: None,
                },
                TypeDeclaration {
                    type_id: 22,
                    type_field: "u64".to_string(),
                    components: None,
                    type_parameters: None,
                },
            ],
            functions: vec![ABIFunction {
                inputs: vec![TypeApplication {
                    name: "arg1".to_string(),
                    type_id: 16,
                    type_arguments: Some(vec![
                        TypeApplication {
                            name: "".to_string(),
                            type_id: 15,
                            type_arguments: None,
                        },
                        TypeApplication {
                            name: "".to_string(),
                            type_id: 7,
                            type_arguments: None,
                        },
                    ]),
                }],
                name: "complex_test".to_string(),
                output: TypeApplication {
                    name: "".to_string(),
                    type_id: 0,
                    type_arguments: None,
                },
            }],
        };

        let the_fun = abi.functions.first().unwrap();
        let types = abi
            .types
            .into_iter()
            .map(|decl| (decl.type_id, decl))
            .collect();

        let selector = resolve_fn_selector(the_fun, &types);

        assert_eq!(selector, "complex_test(s<str[2],b256>((a[b256;2],str[2]),(a[e<s<s<s<str[2]>(s<str[2]>(str[2]))>(a[s<str[2]>(s<str[2]>(str[2]));2])>((s<s<str[2]>(s<str[2]>(str[2]))>(a[s<str[2]>(s<str[2]>(str[2]));2]),s<s<str[2]>(s<str[2]>(str[2]))>(a[s<str[2]>(s<str[2]>(str[2]));2])))>(u64,s<s<s<str[2]>(s<str[2]>(str[2]))>(a[s<str[2]>(s<str[2]>(str[2]));2])>((s<s<str[2]>(s<str[2]>(str[2]))>(a[s<str[2]>(s<str[2]>(str[2]));2]),s<s<str[2]>(s<str[2]>(str[2]))>(a[s<str[2]>(s<str[2]>(str[2]));2]))));1],u32)))");
    }
}
