use std::{collections::HashMap, iter::zip};

use fuel_abi_types::{
    abi::unified_program::{UnifiedTypeApplication, UnifiedTypeDeclaration},
    utils::{extract_array_len, extract_generic_name, extract_str_len, has_tuple_format},
};

use crate::types::{
    errors::{error, Error, Result},
    param_types::{EnumVariants, NamedParamType, ParamType},
};

impl ParamType {
    /// For when you need to convert a ABI JSON's UnifiedTypeApplication into a ParamType.
    ///
    /// # Arguments
    ///
    /// * `type_application`: The UnifiedTypeApplication you wish to convert into a ParamType
    /// * `type_lookup`: A HashMap of UnifiedTypeDeclarations mentioned in the
    ///                  UnifiedTypeApplication where the type id is the key.
    pub fn try_from_type_application(
        type_application: &UnifiedTypeApplication,
        type_lookup: &HashMap<usize, UnifiedTypeDeclaration>,
    ) -> Result<Self> {
        Type::try_from(type_application, type_lookup)?.try_into()
    }
}

#[derive(Debug, Clone)]
struct Type {
    name: String,
    type_field: String,
    generic_params: Vec<Type>,
    components: Vec<Type>,
}

impl Type {
    /// Will recursively drill down the given generic parameters until all types are
    /// resolved.
    ///
    /// # Arguments
    ///
    /// * `type_application`: the type we wish to resolve
    /// * `types`: all types used in the function call
    pub fn try_from(
        type_application: &UnifiedTypeApplication,
        type_lookup: &HashMap<usize, UnifiedTypeDeclaration>,
    ) -> Result<Self> {
        Self::resolve(type_application, type_lookup, &[])
    }

    fn resolve(
        type_application: &UnifiedTypeApplication,
        type_lookup: &HashMap<usize, UnifiedTypeDeclaration>,
        parent_generic_params: &[(usize, Type)],
    ) -> Result<Self> {
        let type_declaration = type_lookup.get(&type_application.type_id).ok_or_else(|| {
            error!(
                Codec,
                "type id {} not found in type lookup", type_application.type_id
            )
        })?;

        if extract_generic_name(&type_declaration.type_field).is_some() {
            let (_, generic_type) = parent_generic_params
                .iter()
                .find(|(id, _)| *id == type_application.type_id)
                .ok_or_else(|| {
                    error!(
                        Codec,
                        "type id {} not found in parent's generic parameters",
                        type_application.type_id
                    )
                })?;

            // The generic will inherit the name from the parent `type_application`
            return Ok(Self {
                name: type_application.name.clone(),
                ..generic_type.clone()
            });
        }

        // Figure out what does the current type do with the inherited generic
        // parameters and reestablish the mapping since the current type might have
        // renamed the inherited generic parameters.
        let generic_params_lookup = Self::determine_generics_for_type(
            type_application,
            type_lookup,
            type_declaration,
            parent_generic_params,
        )?;

        // Resolve the enclosed components (if any) with the newly resolved generic
        // parameters.
        let components = type_declaration
            .components
            .iter()
            .flatten()
            .map(|component| Self::resolve(component, type_lookup, &generic_params_lookup))
            .collect::<Result<Vec<_>>>()?;

        Ok(Type {
            name: type_application.name.clone(),
            type_field: type_declaration.type_field.clone(),
            components,
            generic_params: generic_params_lookup
                .into_iter()
                .map(|(_, ty)| ty)
                .collect(),
        })
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
        type_application: &UnifiedTypeApplication,
        type_lookup: &HashMap<usize, UnifiedTypeDeclaration>,
        type_declaration: &UnifiedTypeDeclaration,
        parent_generic_params: &[(usize, Type)],
    ) -> Result<Vec<(usize, Self)>> {
        match &type_declaration.type_parameters {
            // The presence of type_parameters indicates that the current type
            // (a struct or an enum) defines some generic parameters (i.e. SomeStruct<T, K>).
            Some(params) if !params.is_empty() => {
                // Determine what Types the generics will resolve to.
                let generic_params_from_current_type = type_application
                    .type_arguments
                    .iter()
                    .flatten()
                    .map(|ty| Self::resolve(ty, type_lookup, parent_generic_params))
                    .collect::<Result<Vec<_>>>()?;

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

                Ok(zip(params.clone(), generics_to_use).collect())
            }
            _ => Ok(parent_generic_params.to_vec()),
        }
    }
}

impl TryFrom<Type> for ParamType {
    type Error = Error;

    fn try_from(value: Type) -> Result<Self> {
        (&value).try_into()
    }
}

impl TryFrom<&Type> for ParamType {
    type Error = Error;

    fn try_from(the_type: &Type) -> Result<Self> {
        let matched_param_type = [
            try_primitive,
            try_array,
            try_str_array,
            try_str_slice,
            try_tuple,
            try_vector,
            try_bytes,
            try_std_string,
            try_raw_slice,
            try_enum,
            try_u128,
            try_struct,
        ]
        .into_iter()
        .map(|fun| fun(the_type))
        .flat_map(|result| result.ok().flatten())
        .next();

        matched_param_type.map(Ok).unwrap_or_else(|| {
            Err(error!(
                Codec,
                "type {} couldn't be converted into a ParamType", the_type.type_field
            ))
        })
    }
}

fn convert_into_param_types(coll: &[Type]) -> Result<Vec<ParamType>> {
    coll.iter().map(ParamType::try_from).collect()
}

fn named_param_types(coll: &[Type]) -> Result<Vec<NamedParamType>> {
    coll.iter()
        .map(|ttype| Ok((ttype.name.clone(), ttype.try_into()?)))
        .collect()
}

fn try_struct(the_type: &Type) -> Result<Option<ParamType>> {
    let field = &the_type.type_field;
    if field.starts_with("struct ") {
        let fields = named_param_types(&the_type.components)?;
        let generics = param_types(&the_type.generic_params)?;

        return Ok(Some(ParamType::Struct {
            name: the_type
                .type_field
                .strip_prefix("struct ")
                .expect("has `struct`")
                .to_string(),
            fields,
            generics,
        }));
    }

    Ok(None)
}

fn try_vector(the_type: &Type) -> Result<Option<ParamType>> {
    if !["struct std::vec::Vec", "struct Vec"].contains(&the_type.type_field.as_str()) {
        return Ok(None);
    }

    if the_type.generic_params.len() != 1 {
        return Err(error!(
            Codec,
            "`Vec` must have exactly one generic argument for its type. Found: `{:?}`",
            the_type.generic_params
        ));
    }

    let vec_elem_type = convert_into_param_types(&the_type.generic_params)?.remove(0);

    Ok(Some(ParamType::Vector(Box::new(vec_elem_type))))
}

fn try_u128(the_type: &Type) -> Result<Option<ParamType>> {
    Ok(["struct std::u128::U128", "struct U128"]
        .contains(&the_type.type_field.as_str())
        .then_some(ParamType::U128))
}

fn try_bytes(the_type: &Type) -> Result<Option<ParamType>> {
    Ok(["struct std::bytes::Bytes", "struct Bytes"]
        .contains(&the_type.type_field.as_str())
        .then_some(ParamType::Bytes))
}

fn try_std_string(the_type: &Type) -> Result<Option<ParamType>> {
    Ok(["struct std::string::String", "struct String"]
        .contains(&the_type.type_field.as_str())
        .then_some(ParamType::String))
}

fn try_raw_slice(the_type: &Type) -> Result<Option<ParamType>> {
    Ok((the_type.type_field == "raw untyped slice").then_some(ParamType::RawSlice))
}

fn try_enum(the_type: &Type) -> Result<Option<ParamType>> {
    let field = &the_type.type_field;
    if field.starts_with("enum ") {
        let components = named_param_types(&the_type.components)?;
        let enum_variants = EnumVariants::new(components)?;
        let generics = param_types(&the_type.generic_params)?;

        return Ok(Some(ParamType::Enum {
            name: field.strip_prefix("enum ").expect("has `enum`").to_string(),
            enum_variants,
            generics,
        }));
    }

    Ok(None)
}

fn try_tuple(the_type: &Type) -> Result<Option<ParamType>> {
    let result = if has_tuple_format(&the_type.type_field) {
        let tuple_elements = param_types(&the_type.components)?;
        Some(ParamType::Tuple(tuple_elements))
    } else {
        None
    };

    Ok(result)
}

fn param_types(coll: &[Type]) -> Result<Vec<ParamType>> {
    coll.iter().map(|t| t.try_into()).collect()
}

fn try_str_array(the_type: &Type) -> Result<Option<ParamType>> {
    Ok(extract_str_len(&the_type.type_field).map(ParamType::StringArray))
}

fn try_str_slice(the_type: &Type) -> Result<Option<ParamType>> {
    Ok(if the_type.type_field == "str" {
        Some(ParamType::StringSlice)
    } else {
        None
    })
}

fn try_array(the_type: &Type) -> Result<Option<ParamType>> {
    if let Some(len) = extract_array_len(&the_type.type_field) {
        return match the_type.components.as_slice() {
            [single_type] => {
                let array_type = single_type.try_into()?;
                Ok(Some(ParamType::Array(Box::new(array_type), len)))
            }
            _ => Err(error!(
                Codec,
                "array must have elements of exactly one type. Array types: {:?}",
                the_type.components
            )),
        };
    }
    Ok(None)
}

fn try_primitive(the_type: &Type) -> Result<Option<ParamType>> {
    let result = match the_type.type_field.as_str() {
        "bool" => Some(ParamType::Bool),
        "u8" => Some(ParamType::U8),
        "u16" => Some(ParamType::U16),
        "u32" => Some(ParamType::U32),
        "u64" => Some(ParamType::U64),
        "u256" => Some(ParamType::U256),
        "b256" => Some(ParamType::B256),
        "()" => Some(ParamType::Unit),
        "str" => Some(ParamType::StringSlice),
        _ => None,
    };

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handles_simple_types() -> Result<()> {
        let parse_param_type = |type_field: &str| {
            let type_application = UnifiedTypeApplication {
                name: "".to_string(),
                type_id: 0,
                type_arguments: None,
            };

            let declarations = [UnifiedTypeDeclaration {
                type_id: 0,
                type_field: type_field.to_string(),
                components: None,
                type_parameters: None,
            }];

            let type_lookup = declarations
                .into_iter()
                .map(|decl| (decl.type_id, decl))
                .collect::<HashMap<_, _>>();

            ParamType::try_from_type_application(&type_application, &type_lookup)
        };

        assert_eq!(parse_param_type("()")?, ParamType::Unit);
        assert_eq!(parse_param_type("bool")?, ParamType::Bool);
        assert_eq!(parse_param_type("u8")?, ParamType::U8);
        assert_eq!(parse_param_type("u16")?, ParamType::U16);
        assert_eq!(parse_param_type("u32")?, ParamType::U32);
        assert_eq!(parse_param_type("u64")?, ParamType::U64);
        assert_eq!(parse_param_type("u256")?, ParamType::U256);
        assert_eq!(parse_param_type("b256")?, ParamType::B256);
        assert_eq!(parse_param_type("str[21]")?, ParamType::StringArray(21));
        assert_eq!(parse_param_type("str")?, ParamType::StringSlice);

        Ok(())
    }

    #[test]
    fn handles_arrays() -> Result<()> {
        // given
        let type_application = UnifiedTypeApplication {
            name: "".to_string(),
            type_id: 0,
            type_arguments: None,
        };

        let declarations = [
            UnifiedTypeDeclaration {
                type_id: 0,
                type_field: "[_; 10]".to_string(),
                components: Some(vec![UnifiedTypeApplication {
                    name: "__array_element".to_string(),
                    type_id: 1,
                    type_arguments: None,
                }]),
                type_parameters: None,
            },
            UnifiedTypeDeclaration {
                type_id: 1,
                type_field: "u8".to_string(),
                components: None,
                type_parameters: None,
            },
        ];

        let type_lookup = declarations
            .into_iter()
            .map(|decl| (decl.type_id, decl))
            .collect::<HashMap<_, _>>();

        // when
        let result = ParamType::try_from_type_application(&type_application, &type_lookup)?;

        // then
        assert_eq!(result, ParamType::Array(Box::new(ParamType::U8), 10));

        Ok(())
    }

    #[test]
    fn handles_vectors() -> Result<()> {
        // given
        let declarations = [
            UnifiedTypeDeclaration {
                type_id: 1,
                type_field: "generic T".to_string(),
                components: None,
                type_parameters: None,
            },
            UnifiedTypeDeclaration {
                type_id: 2,
                type_field: "raw untyped ptr".to_string(),
                components: None,
                type_parameters: None,
            },
            UnifiedTypeDeclaration {
                type_id: 3,
                type_field: "struct std::vec::RawVec".to_string(),
                components: Some(vec![
                    UnifiedTypeApplication {
                        name: "ptr".to_string(),
                        type_id: 2,
                        type_arguments: None,
                    },
                    UnifiedTypeApplication {
                        name: "cap".to_string(),
                        type_id: 5,
                        type_arguments: None,
                    },
                ]),
                type_parameters: Some(vec![1]),
            },
            UnifiedTypeDeclaration {
                type_id: 4,
                type_field: "struct std::vec::Vec".to_string(),
                components: Some(vec![
                    UnifiedTypeApplication {
                        name: "buf".to_string(),
                        type_id: 3,
                        type_arguments: Some(vec![UnifiedTypeApplication {
                            name: "".to_string(),
                            type_id: 1,
                            type_arguments: None,
                        }]),
                    },
                    UnifiedTypeApplication {
                        name: "len".to_string(),
                        type_id: 5,
                        type_arguments: None,
                    },
                ]),
                type_parameters: Some(vec![1]),
            },
            UnifiedTypeDeclaration {
                type_id: 5,
                type_field: "u64".to_string(),
                components: None,
                type_parameters: None,
            },
            UnifiedTypeDeclaration {
                type_id: 6,
                type_field: "u8".to_string(),
                components: None,
                type_parameters: None,
            },
        ];

        let type_application = UnifiedTypeApplication {
            name: "arg".to_string(),
            type_id: 4,
            type_arguments: Some(vec![UnifiedTypeApplication {
                name: "".to_string(),
                type_id: 6,
                type_arguments: None,
            }]),
        };

        let type_lookup = declarations
            .into_iter()
            .map(|decl| (decl.type_id, decl))
            .collect::<HashMap<_, _>>();

        // when
        let result = ParamType::try_from_type_application(&type_application, &type_lookup)?;

        // then
        assert_eq!(result, ParamType::Vector(Box::new(ParamType::U8)));

        Ok(())
    }

    #[test]
    fn handles_structs() -> Result<()> {
        // given
        let declarations = [
            UnifiedTypeDeclaration {
                type_id: 1,
                type_field: "generic T".to_string(),
                components: None,
                type_parameters: None,
            },
            UnifiedTypeDeclaration {
                type_id: 2,
                type_field: "struct SomeStruct".to_string(),
                components: Some(vec![UnifiedTypeApplication {
                    name: "field".to_string(),
                    type_id: 1,
                    type_arguments: None,
                }]),
                type_parameters: Some(vec![1]),
            },
            UnifiedTypeDeclaration {
                type_id: 3,
                type_field: "u8".to_string(),
                components: None,
                type_parameters: None,
            },
        ];

        let type_application = UnifiedTypeApplication {
            name: "arg".to_string(),
            type_id: 2,
            type_arguments: Some(vec![UnifiedTypeApplication {
                name: "".to_string(),
                type_id: 3,
                type_arguments: None,
            }]),
        };

        let type_lookup = declarations
            .into_iter()
            .map(|decl| (decl.type_id, decl))
            .collect::<HashMap<_, _>>();

        // when
        let result = ParamType::try_from_type_application(&type_application, &type_lookup)?;

        // then
        assert_eq!(
            result,
            ParamType::Struct {
                name: "SomeStruct".to_string(),
                fields: vec![("field".to_string(), ParamType::U8)],
                generics: vec![ParamType::U8]
            }
        );

        Ok(())
    }

    #[test]
    fn handles_enums() -> Result<()> {
        // given
        let declarations = [
            UnifiedTypeDeclaration {
                type_id: 1,
                type_field: "generic T".to_string(),
                components: None,
                type_parameters: None,
            },
            UnifiedTypeDeclaration {
                type_id: 2,
                type_field: "enum SomeEnum".to_string(),
                components: Some(vec![UnifiedTypeApplication {
                    name: "Variant".to_string(),
                    type_id: 1,
                    type_arguments: None,
                }]),
                type_parameters: Some(vec![1]),
            },
            UnifiedTypeDeclaration {
                type_id: 3,
                type_field: "u8".to_string(),
                components: None,
                type_parameters: None,
            },
        ];

        let type_application = UnifiedTypeApplication {
            name: "arg".to_string(),
            type_id: 2,
            type_arguments: Some(vec![UnifiedTypeApplication {
                name: "".to_string(),
                type_id: 3,
                type_arguments: None,
            }]),
        };

        let type_lookup = declarations
            .into_iter()
            .map(|decl| (decl.type_id, decl))
            .collect::<HashMap<_, _>>();

        // when
        let result = ParamType::try_from_type_application(&type_application, &type_lookup)?;

        // then
        assert_eq!(
            result,
            ParamType::Enum {
                name: "SomeEnum".to_string(),
                enum_variants: EnumVariants::new(vec![("Variant".to_string(), ParamType::U8)])?,
                generics: vec![ParamType::U8]
            }
        );

        Ok(())
    }

    #[test]
    fn handles_tuples() -> Result<()> {
        // given
        let declarations = [
            UnifiedTypeDeclaration {
                type_id: 1,
                type_field: "(_, _)".to_string(),
                components: Some(vec![
                    UnifiedTypeApplication {
                        name: "__tuple_element".to_string(),
                        type_id: 3,
                        type_arguments: None,
                    },
                    UnifiedTypeApplication {
                        name: "__tuple_element".to_string(),
                        type_id: 2,
                        type_arguments: None,
                    },
                ]),
                type_parameters: None,
            },
            UnifiedTypeDeclaration {
                type_id: 2,
                type_field: "str[15]".to_string(),
                components: None,
                type_parameters: None,
            },
            UnifiedTypeDeclaration {
                type_id: 3,
                type_field: "u8".to_string(),
                components: None,
                type_parameters: None,
            },
        ];

        let type_application = UnifiedTypeApplication {
            name: "arg".to_string(),
            type_id: 1,
            type_arguments: None,
        };
        let type_lookup = declarations
            .into_iter()
            .map(|decl| (decl.type_id, decl))
            .collect::<HashMap<_, _>>();

        // when
        let result = ParamType::try_from_type_application(&type_application, &type_lookup)?;

        // then
        assert_eq!(
            result,
            ParamType::Tuple(vec![ParamType::U8, ParamType::StringArray(15)])
        );

        Ok(())
    }

    #[test]
    fn ultimate_example() -> Result<()> {
        // given
        let declarations = [
            UnifiedTypeDeclaration {
                type_id: 1,
                type_field: "(_, _)".to_string(),
                components: Some(vec![
                    UnifiedTypeApplication {
                        name: "__tuple_element".to_string(),
                        type_id: 11,
                        type_arguments: None,
                    },
                    UnifiedTypeApplication {
                        name: "__tuple_element".to_string(),
                        type_id: 11,
                        type_arguments: None,
                    },
                ]),
                type_parameters: None,
            },
            UnifiedTypeDeclaration {
                type_id: 2,
                type_field: "(_, _)".to_string(),
                components: Some(vec![
                    UnifiedTypeApplication {
                        name: "__tuple_element".to_string(),
                        type_id: 4,
                        type_arguments: None,
                    },
                    UnifiedTypeApplication {
                        name: "__tuple_element".to_string(),
                        type_id: 24,
                        type_arguments: None,
                    },
                ]),
                type_parameters: None,
            },
            UnifiedTypeDeclaration {
                type_id: 3,
                type_field: "(_, _)".to_string(),
                components: Some(vec![
                    UnifiedTypeApplication {
                        name: "__tuple_element".to_string(),
                        type_id: 5,
                        type_arguments: None,
                    },
                    UnifiedTypeApplication {
                        name: "__tuple_element".to_string(),
                        type_id: 13,
                        type_arguments: None,
                    },
                ]),
                type_parameters: None,
            },
            UnifiedTypeDeclaration {
                type_id: 4,
                type_field: "[_; 1]".to_string(),
                components: Some(vec![UnifiedTypeApplication {
                    name: "__array_element".to_string(),
                    type_id: 8,
                    type_arguments: Some(vec![UnifiedTypeApplication {
                        name: "".to_string(),
                        type_id: 22,
                        type_arguments: Some(vec![UnifiedTypeApplication {
                            name: "".to_string(),
                            type_id: 21,
                            type_arguments: Some(vec![UnifiedTypeApplication {
                                name: "".to_string(),
                                type_id: 18,
                                type_arguments: Some(vec![UnifiedTypeApplication {
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
            UnifiedTypeDeclaration {
                type_id: 5,
                type_field: "[_; 2]".to_string(),
                components: Some(vec![UnifiedTypeApplication {
                    name: "__array_element".to_string(),
                    type_id: 14,
                    type_arguments: None,
                }]),
                type_parameters: None,
            },
            UnifiedTypeDeclaration {
                type_id: 6,
                type_field: "[_; 2]".to_string(),
                components: Some(vec![UnifiedTypeApplication {
                    name: "__array_element".to_string(),
                    type_id: 10,
                    type_arguments: None,
                }]),
                type_parameters: None,
            },
            UnifiedTypeDeclaration {
                type_id: 7,
                type_field: "b256".to_string(),
                components: None,
                type_parameters: None,
            },
            UnifiedTypeDeclaration {
                type_id: 8,
                type_field: "enum EnumWGeneric".to_string(),
                components: Some(vec![
                    UnifiedTypeApplication {
                        name: "A".to_string(),
                        type_id: 25,
                        type_arguments: None,
                    },
                    UnifiedTypeApplication {
                        name: "B".to_string(),
                        type_id: 12,
                        type_arguments: None,
                    },
                ]),
                type_parameters: Some(vec![12]),
            },
            UnifiedTypeDeclaration {
                type_id: 9,
                type_field: "generic K".to_string(),
                components: None,
                type_parameters: None,
            },
            UnifiedTypeDeclaration {
                type_id: 10,
                type_field: "generic L".to_string(),
                components: None,
                type_parameters: None,
            },
            UnifiedTypeDeclaration {
                type_id: 11,
                type_field: "generic M".to_string(),
                components: None,
                type_parameters: None,
            },
            UnifiedTypeDeclaration {
                type_id: 12,
                type_field: "generic N".to_string(),
                components: None,
                type_parameters: None,
            },
            UnifiedTypeDeclaration {
                type_id: 13,
                type_field: "generic T".to_string(),
                components: None,
                type_parameters: None,
            },
            UnifiedTypeDeclaration {
                type_id: 14,
                type_field: "generic U".to_string(),
                components: None,
                type_parameters: None,
            },
            UnifiedTypeDeclaration {
                type_id: 15,
                type_field: "raw untyped ptr".to_string(),
                components: None,
                type_parameters: None,
            },
            UnifiedTypeDeclaration {
                type_id: 16,
                type_field: "str[2]".to_string(),
                components: None,
                type_parameters: None,
            },
            UnifiedTypeDeclaration {
                type_id: 17,
                type_field: "struct MegaExample".to_string(),
                components: Some(vec![
                    UnifiedTypeApplication {
                        name: "a".to_string(),
                        type_id: 3,
                        type_arguments: None,
                    },
                    UnifiedTypeApplication {
                        name: "b".to_string(),
                        type_id: 23,
                        type_arguments: Some(vec![UnifiedTypeApplication {
                            name: "".to_string(),
                            type_id: 2,
                            type_arguments: None,
                        }]),
                    },
                ]),
                type_parameters: Some(vec![13, 14]),
            },
            UnifiedTypeDeclaration {
                type_id: 18,
                type_field: "struct PassTheGenericOn".to_string(),
                components: Some(vec![UnifiedTypeApplication {
                    name: "one".to_string(),
                    type_id: 20,
                    type_arguments: Some(vec![UnifiedTypeApplication {
                        name: "".to_string(),
                        type_id: 9,
                        type_arguments: None,
                    }]),
                }]),
                type_parameters: Some(vec![9]),
            },
            UnifiedTypeDeclaration {
                type_id: 19,
                type_field: "struct std::vec::RawVec".to_string(),
                components: Some(vec![
                    UnifiedTypeApplication {
                        name: "ptr".to_string(),
                        type_id: 15,
                        type_arguments: None,
                    },
                    UnifiedTypeApplication {
                        name: "cap".to_string(),
                        type_id: 25,
                        type_arguments: None,
                    },
                ]),
                type_parameters: Some(vec![13]),
            },
            UnifiedTypeDeclaration {
                type_id: 20,
                type_field: "struct SimpleGeneric".to_string(),
                components: Some(vec![UnifiedTypeApplication {
                    name: "single_generic_param".to_string(),
                    type_id: 13,
                    type_arguments: None,
                }]),
                type_parameters: Some(vec![13]),
            },
            UnifiedTypeDeclaration {
                type_id: 21,
                type_field: "struct StructWArrayGeneric".to_string(),
                components: Some(vec![UnifiedTypeApplication {
                    name: "a".to_string(),
                    type_id: 6,
                    type_arguments: None,
                }]),
                type_parameters: Some(vec![10]),
            },
            UnifiedTypeDeclaration {
                type_id: 22,
                type_field: "struct StructWTupleGeneric".to_string(),
                components: Some(vec![UnifiedTypeApplication {
                    name: "a".to_string(),
                    type_id: 1,
                    type_arguments: None,
                }]),
                type_parameters: Some(vec![11]),
            },
            UnifiedTypeDeclaration {
                type_id: 23,
                type_field: "struct std::vec::Vec".to_string(),
                components: Some(vec![
                    UnifiedTypeApplication {
                        name: "buf".to_string(),
                        type_id: 19,
                        type_arguments: Some(vec![UnifiedTypeApplication {
                            name: "".to_string(),
                            type_id: 13,
                            type_arguments: None,
                        }]),
                    },
                    UnifiedTypeApplication {
                        name: "len".to_string(),
                        type_id: 25,
                        type_arguments: None,
                    },
                ]),
                type_parameters: Some(vec![13]),
            },
            UnifiedTypeDeclaration {
                type_id: 24,
                type_field: "u32".to_string(),
                components: None,
                type_parameters: None,
            },
            UnifiedTypeDeclaration {
                type_id: 25,
                type_field: "u64".to_string(),
                components: None,
                type_parameters: None,
            },
        ];

        let type_lookup = declarations
            .into_iter()
            .map(|decl| (decl.type_id, decl))
            .collect::<HashMap<_, _>>();

        let type_application = UnifiedTypeApplication {
            name: "arg1".to_string(),
            type_id: 17,
            type_arguments: Some(vec![
                UnifiedTypeApplication {
                    name: "".to_string(),
                    type_id: 16,
                    type_arguments: None,
                },
                UnifiedTypeApplication {
                    name: "".to_string(),
                    type_id: 7,
                    type_arguments: None,
                },
            ]),
        };

        // when
        let result = ParamType::try_from_type_application(&type_application, &type_lookup)?;

        // then
        let expected_param_type = {
            let fields = vec![(
                "one".to_string(),
                ParamType::Struct {
                    name: "SimpleGeneric".to_string(),
                    fields: vec![(
                        "single_generic_param".to_string(),
                        ParamType::StringArray(2),
                    )],
                    generics: vec![ParamType::StringArray(2)],
                },
            )];
            let pass_the_generic_on = ParamType::Struct {
                name: "PassTheGenericOn".to_string(),
                fields,
                generics: vec![ParamType::StringArray(2)],
            };

            let fields = vec![(
                "a".to_string(),
                ParamType::Array(Box::from(pass_the_generic_on.clone()), 2),
            )];
            let struct_w_array_generic = ParamType::Struct {
                name: "StructWArrayGeneric".to_string(),
                fields,
                generics: vec![pass_the_generic_on],
            };

            let fields = vec![(
                "a".to_string(),
                ParamType::Tuple(vec![
                    struct_w_array_generic.clone(),
                    struct_w_array_generic.clone(),
                ]),
            )];
            let struct_w_tuple_generic = ParamType::Struct {
                name: "StructWTupleGeneric".to_string(),
                fields,
                generics: vec![struct_w_array_generic],
            };

            let types = vec![
                ("A".to_string(), ParamType::U64),
                ("B".to_string(), struct_w_tuple_generic.clone()),
            ];
            let fields = vec![
                (
                    "a".to_string(),
                    ParamType::Tuple(vec![
                        ParamType::Array(Box::from(ParamType::B256), 2),
                        ParamType::StringArray(2),
                    ]),
                ),
                (
                    "b".to_string(),
                    ParamType::Vector(Box::from(ParamType::Tuple(vec![
                        ParamType::Array(
                            Box::from(ParamType::Enum {
                                name: "EnumWGeneric".to_string(),
                                enum_variants: EnumVariants::new(types).unwrap(),
                                generics: vec![struct_w_tuple_generic],
                            }),
                            1,
                        ),
                        ParamType::U32,
                    ]))),
                ),
            ];
            ParamType::Struct {
                name: "MegaExample".to_string(),
                fields,
                generics: vec![ParamType::StringArray(2), ParamType::B256],
            }
        };

        assert_eq!(result, expected_param_type);

        Ok(())
    }

    #[test]
    fn try_vector_correctly_resolves_param_type() {
        let the_type = given_generic_type_with_path("std::vec::Vec");

        let param_type = try_vector(&the_type).unwrap().unwrap();

        assert_eq!(param_type, ParamType::Vector(Box::new(ParamType::U8)));
    }

    #[test]
    fn try_bytes_correctly_resolves_param_type() {
        let the_type = given_type_with_path("std::bytes::Bytes");

        let param_type = try_bytes(&the_type).unwrap().unwrap();

        assert_eq!(param_type, ParamType::Bytes);
    }

    #[test]
    fn try_raw_slice_correctly_resolves_param_type() {
        let the_type = Type {
            name: "".to_string(),
            type_field: "raw untyped slice".to_string(),
            generic_params: vec![],
            components: vec![],
        };

        let param_type = try_raw_slice(&the_type).unwrap().unwrap();

        assert_eq!(param_type, ParamType::RawSlice);
    }

    #[test]
    fn try_std_string_correctly_resolves_param_type() {
        let the_type = given_type_with_path("std::string::String");

        let param_type = try_std_string(&the_type).unwrap().unwrap();

        assert_eq!(param_type, ParamType::String);
    }

    fn given_type_with_path(path: &str) -> Type {
        Type {
            name: "".to_string(),
            type_field: format!("struct {path}"),
            generic_params: vec![],
            components: vec![],
        }
    }

    fn given_generic_type_with_path(path: &str) -> Type {
        Type {
            name: "".to_string(),
            type_field: format!("struct {path}"),
            generic_params: vec![Type {
                name: "".to_string(),
                type_field: "u8".to_string(),
                generic_params: vec![],
                components: vec![],
            }],
            components: vec![],
        }
    }
}
