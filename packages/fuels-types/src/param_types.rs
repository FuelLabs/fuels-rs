use crate::constants::WORD_SIZE;
use crate::enum_variants::{EnumVariants, NoVariants};
use crate::errors::Error;
use crate::utils::custom_type_name;
use crate::utils::{
    extract_array_len, extract_generic_name, extract_str_len, has_enum_format, has_struct_format,
    has_tuple_format,
};
use crate::{TypeApplication, TypeDeclaration};
use itertools::Itertools;
use std::collections::HashMap;
use std::iter::zip;
use strum_macros::EnumString;

#[derive(Debug, Clone, EnumString, PartialEq, Eq)]
#[strum(ascii_case_insensitive)]
pub enum ParamType {
    U8,
    U16,
    U32,
    U64,
    Bool,
    Byte,
    B256,
    // The Unit paramtype is used for unit variants in Enums. The corresponding type field is `()`,
    // similar to Rust.
    Unit,
    Array(Box<ParamType>, usize),
    Vector(Box<ParamType>),
    #[strum(serialize = "str")]
    String(usize),
    #[strum(disabled)]
    Struct {
        fields: Vec<ParamType>,
        generics: Vec<ParamType>,
    },
    #[strum(disabled)]
    Enum {
        variants: EnumVariants,
        generics: Vec<ParamType>,
    },
    Tuple(Vec<ParamType>),
}

impl Default for ParamType {
    fn default() -> Self {
        ParamType::U8
    }
}

pub enum ReturnLocation {
    Return,
    ReturnData,
}

impl ParamType {
    // Depending on the type, the returned value will be stored
    // either in `Return` or `ReturnData`.
    pub fn get_return_location(&self) -> ReturnLocation {
        match self {
            Self::Unit | Self::U8 | Self::U16 | Self::U32 | Self::U64 | Self::Bool => {
                ReturnLocation::Return
            }

            _ => ReturnLocation::ReturnData,
        }
    }

    /// Calculates the number of `WORD`s the VM expects this parameter to be encoded in.
    pub fn compute_encoding_width(&self) -> usize {
        const fn count_words(bytes: usize) -> usize {
            let q = bytes / WORD_SIZE;
            let r = bytes % WORD_SIZE;
            match r == 0 {
                true => q,
                false => q + 1,
            }
        }

        match &self {
            ParamType::Unit
            | ParamType::U8
            | ParamType::U16
            | ParamType::U32
            | ParamType::U64
            | ParamType::Bool
            | ParamType::Byte => 1,
            ParamType::Vector(_) => 3,
            ParamType::B256 => 4,
            ParamType::Array(param, count) => param.compute_encoding_width() * count,
            ParamType::String(len) => count_words(*len),
            ParamType::Struct { fields, .. } => {
                fields.iter().map(|p| p.compute_encoding_width()).sum()
            }
            ParamType::Enum { variants, .. } => variants.compute_encoding_width_of_enum(),
            ParamType::Tuple(params) => params.iter().map(|p| p.compute_encoding_width()).sum(),
        }
    }
    /// For when you need to convert a ABI JSON's TypeApplication into a ParamType.
    ///
    /// # Arguments
    ///
    /// * `type_application`: The TypeApplication you wish to convert into a ParamType
    /// * `type_lookup`: A HashMap of TypeDeclarations mentioned in the
    ///                  TypeApplication where the type id is the key.
    pub fn try_from_type_application(
        type_application: &TypeApplication,
        type_lookup: &HashMap<usize, TypeDeclaration>,
    ) -> Result<Self, Error> {
        Type::from(type_application, type_lookup).try_into()
    }
}

#[derive(Debug, Clone)]
struct Type {
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
    pub fn from(
        type_application: &TypeApplication,
        type_lookup: &HashMap<usize, TypeDeclaration>,
    ) -> Self {
        Self::resolve(type_application, type_lookup, &[])
    }

    fn resolve(
        type_application: &TypeApplication,
        type_lookup: &HashMap<usize, TypeDeclaration>,
        parent_generic_params: &[(usize, Type)],
    ) -> Self {
        let type_decl = type_lookup.get(&type_application.type_id).unwrap();

        if extract_generic_name(&type_decl.type_field).is_some() {
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
            Self::determine_generics_for_type(type_application, type_lookup, parent_generic_params);

        // Resolve the enclosed components (if any) with the newly resolved generic
        // parameters.
        let components = type_decl
            .components
            .iter()
            .flatten()
            .map(|component| Self::resolve(component, type_lookup, &generic_params_lookup))
            .collect_vec();

        Type {
            type_field: type_decl.type_field.clone(),
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
    ) -> Vec<(usize, Self)> {
        let type_decl = types.get(&type_application.type_id).unwrap();
        match &type_decl.type_parameters {
            // The presence of type_parameters indicates that the current type
            // (a struct or an enum) defines some generic parameters (i.e. SomeStruct<T, K>).
            Some(params) if !params.is_empty() => {
                // Determine what Types the generics will resolve to.
                let generic_params_from_current_type = type_application
                    .type_arguments
                    .iter()
                    .flatten()
                    .map(|ty| Self::resolve(ty, types, parent_generic_params))
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
}

impl TryFrom<Type> for ParamType {
    type Error = Error;

    fn try_from(value: Type) -> Result<Self, Self::Error> {
        (&value).try_into()
    }
}

impl TryFrom<&Type> for ParamType {
    type Error = Error;

    fn try_from(the_type: &Type) -> Result<Self, Self::Error> {
        let mut matching_param_types = [
            try_primitive,
            try_array,
            try_str,
            try_tuple,
            try_vector,
            try_enum,
            try_struct,
        ]
        .into_iter()
        .map(|fun| fun(the_type))
        .flatten_ok()
        .collect::<Result<Vec<_>, _>>()?;

        if matching_param_types.is_empty() {
            Err(Error::InvalidType(
                "Type didn't match against any known ParamTypes".to_string(),
            ))
        } else {
            Ok(matching_param_types.remove(0))
        }
    }
}

fn try_struct(the_type: &Type) -> Result<Option<ParamType>, Error> {
    let result = if has_struct_format(&the_type.type_field) {
        let generics = to_param_types(&the_type.generic_params)?;

        let fields = to_param_types(&the_type.components)?;
        Some(ParamType::Struct { fields, generics })
    } else {
        None
    };

    Ok(result)
}

fn try_vector(the_type: &Type) -> Result<Option<ParamType>, Error> {
    if has_struct_format(&the_type.type_field) && custom_type_name(&the_type.type_field)? == "Vec" {
        if the_type.generic_params.len() != 1 {
            return Err(Error::InvalidType(format!(
                "Vec must have exactly one generic argument for its type. Found: {:?}",
                the_type.generic_params
            )));
        }

        let vec_elem_type = to_param_types(&the_type.generic_params)?.remove(0);

        return Ok(Some(ParamType::Vector(Box::new(vec_elem_type))));
    }

    Ok(None)
}

fn try_enum(the_type: &Type) -> Result<Option<ParamType>, Error> {
    let result = if has_enum_format(&the_type.type_field) {
        let generics = to_param_types(&the_type.generic_params)?;

        let components = to_param_types(&the_type.components)?;
        let variants = EnumVariants::new(components)?;

        Some(ParamType::Enum { variants, generics })
    } else {
        None
    };

    Ok(result)
}

fn try_tuple(the_type: &Type) -> Result<Option<ParamType>, Error> {
    let result = if has_tuple_format(&the_type.type_field) {
        let tuple_elements = to_param_types(&the_type.components)?;
        Some(ParamType::Tuple(tuple_elements))
    } else {
        None
    };

    Ok(result)
}

fn to_param_types(coll: &[Type]) -> Result<Vec<ParamType>, Error> {
    coll.iter().map(|e| e.try_into()).collect()
}

fn try_str(the_type: &Type) -> Result<Option<ParamType>, Error> {
    Ok(extract_str_len(&the_type.type_field).map(ParamType::String))
}

fn try_array(the_type: &Type) -> Result<Option<ParamType>, Error> {
    if let Some(len) = extract_array_len(&the_type.type_field) {
        if the_type.components.len() != 1 {}

        return match the_type.components.as_slice() {
            [single_type] => {
                let array_type = single_type.try_into()?;
                Ok(Some(ParamType::Array(Box::new(array_type), len)))
            }
            _ => Err(Error::InvalidType(format!(
                "An array must have elements of exactly one type. Array types: {:?}",
                the_type.components
            ))),
        };
    }
    Ok(None)
}

fn try_primitive(the_type: &Type) -> Result<Option<ParamType>, Error> {
    let result = match the_type.type_field.as_str() {
        "byte" => Some(ParamType::Byte),
        "bool" => Some(ParamType::Bool),
        "u8" => Some(ParamType::U8),
        "u16" => Some(ParamType::U16),
        "u32" => Some(ParamType::U32),
        "u64" => Some(ParamType::U64),
        "b256" => Some(ParamType::B256),
        "()" => Some(ParamType::Unit),
        _ => None,
    };

    Ok(result)
}

impl From<NoVariants> for Error {
    fn from(err: NoVariants) -> Self {
        Error::InvalidType(format!("{}", err))
    }
}

#[cfg(test)]
mod tests {
    const WIDTH_OF_B256: usize = 4;
    const WIDTH_OF_U32: usize = 1;
    const WIDTH_OF_BOOL: usize = 1;

    use super::*;
    use crate::param_types::ParamType;

    #[test]
    fn array_size_dependent_on_num_of_elements() {
        const NUM_ELEMENTS: usize = 11;
        let param = ParamType::Array(Box::new(ParamType::B256), NUM_ELEMENTS);

        let width = param.compute_encoding_width();

        let expected = NUM_ELEMENTS * WIDTH_OF_B256;
        assert_eq!(expected, width);
    }

    #[test]
    fn string_size_dependent_on_num_of_elements() {
        const NUM_ASCII_CHARS: usize = 9;
        let param = ParamType::String(NUM_ASCII_CHARS);

        let width = param.compute_encoding_width();

        // 2 WORDS or 16 B are enough to fit 9 ascii chars
        assert_eq!(2, width);
    }

    #[test]
    fn structs_are_just_all_elements_combined() {
        let inner_struct = ParamType::Struct {
            fields: vec![ParamType::U32, ParamType::U32],
            generics: vec![],
        };

        let a_struct = ParamType::Struct {
            fields: vec![ParamType::B256, ParamType::Bool, inner_struct],
            generics: vec![],
        };

        let width = a_struct.compute_encoding_width();

        const INNER_STRUCT_WIDTH: usize = WIDTH_OF_U32 * 2;
        const EXPECTED_WIDTH: usize = WIDTH_OF_B256 + WIDTH_OF_BOOL + INNER_STRUCT_WIDTH;
        assert_eq!(EXPECTED_WIDTH, width);
    }

    #[test]
    fn enums_are_as_big_as_their_biggest_variant_plus_a_word() -> Result<(), Error> {
        let inner_struct = ParamType::Struct {
            fields: vec![ParamType::B256],
            generics: vec![],
        };
        let param = ParamType::Enum {
            variants: EnumVariants::new(vec![ParamType::U32, inner_struct])?,
            generics: vec![],
        };

        let width = param.compute_encoding_width();

        const INNER_STRUCT_SIZE: usize = WIDTH_OF_B256;
        const EXPECTED_WIDTH: usize = INNER_STRUCT_SIZE + 1;
        assert_eq!(EXPECTED_WIDTH, width);
        Ok(())
    }

    #[test]
    fn tuples_are_just_all_elements_combined() {
        let inner_tuple = ParamType::Tuple(vec![ParamType::B256]);
        let param = ParamType::Tuple(vec![ParamType::U32, inner_tuple]);

        let width = param.compute_encoding_width();

        const INNER_TUPLE_WIDTH: usize = WIDTH_OF_B256;
        const EXPECTED_WIDTH: usize = WIDTH_OF_U32 + INNER_TUPLE_WIDTH;
        assert_eq!(EXPECTED_WIDTH, width);
    }

    #[test]
    fn handles_simple_types() -> Result<(), Error> {
        let parse_param_type = |type_field: &str| {
            let type_application = TypeApplication {
                name: "".to_string(),
                type_id: 0,
                type_arguments: None,
            };

            let declarations = [TypeDeclaration {
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

        assert_eq!(parse_param_type("u8")?, ParamType::U8);
        assert_eq!(parse_param_type("u16")?, ParamType::U16);
        assert_eq!(parse_param_type("u32")?, ParamType::U32);
        assert_eq!(parse_param_type("u64")?, ParamType::U64);
        assert_eq!(parse_param_type("bool")?, ParamType::Bool);
        assert_eq!(parse_param_type("byte")?, ParamType::Byte);
        assert_eq!(parse_param_type("b256")?, ParamType::B256);
        assert_eq!(parse_param_type("()")?, ParamType::Unit);
        assert_eq!(parse_param_type("str[21]")?, ParamType::String(21));

        Ok(())
    }

    #[test]
    fn handles_arrays() -> Result<(), Error> {
        // given
        let type_application = TypeApplication {
            name: "".to_string(),
            type_id: 0,
            type_arguments: None,
        };

        let declarations = [
            TypeDeclaration {
                type_id: 0,
                type_field: "[_; 10]".to_string(),
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
    fn handles_vectors() -> Result<(), Error> {
        // given
        let declarations = [
            TypeDeclaration {
                type_id: 1,
                type_field: "generic T".to_string(),
                components: None,
                type_parameters: None,
            },
            TypeDeclaration {
                type_id: 2,
                type_field: "struct RawVec".to_string(),
                components: Some(vec![
                    TypeApplication {
                        name: "ptr".to_string(),
                        type_id: 4,
                        type_arguments: None,
                    },
                    TypeApplication {
                        name: "cap".to_string(),
                        type_id: 4,
                        type_arguments: None,
                    },
                ]),
                type_parameters: Some(vec![1]),
            },
            TypeDeclaration {
                type_id: 3,
                type_field: "struct Vec".to_string(),
                components: Some(vec![
                    TypeApplication {
                        name: "buf".to_string(),
                        type_id: 2,
                        type_arguments: Some(vec![TypeApplication {
                            name: "".to_string(),
                            type_id: 1,
                            type_arguments: None,
                        }]),
                    },
                    TypeApplication {
                        name: "len".to_string(),
                        type_id: 4,
                        type_arguments: None,
                    },
                ]),
                type_parameters: Some(vec![1]),
            },
            TypeDeclaration {
                type_id: 4,
                type_field: "u64".to_string(),
                components: None,
                type_parameters: None,
            },
            TypeDeclaration {
                type_id: 5,
                type_field: "u8".to_string(),
                components: None,
                type_parameters: None,
            },
        ];

        let type_application = TypeApplication {
            name: "arg".to_string(),
            type_id: 3,
            type_arguments: Some(vec![TypeApplication {
                name: "".to_string(),
                type_id: 5,
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
    fn handles_structs() -> Result<(), Error> {
        // given
        let declarations = [
            TypeDeclaration {
                type_id: 1,
                type_field: "generic T".to_string(),
                components: None,
                type_parameters: None,
            },
            TypeDeclaration {
                type_id: 2,
                type_field: "struct SomeStruct".to_string(),
                components: Some(vec![TypeApplication {
                    name: "field".to_string(),
                    type_id: 1,
                    type_arguments: None,
                }]),
                type_parameters: Some(vec![1]),
            },
            TypeDeclaration {
                type_id: 3,
                type_field: "u8".to_string(),
                components: None,
                type_parameters: None,
            },
        ];

        let type_application = TypeApplication {
            name: "arg".to_string(),
            type_id: 2,
            type_arguments: Some(vec![TypeApplication {
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
                fields: vec![ParamType::U8],
                generics: vec![ParamType::U8]
            }
        );

        Ok(())
    }

    #[test]
    fn handles_enums() -> Result<(), Error> {
        // given
        let declarations = [
            TypeDeclaration {
                type_id: 1,
                type_field: "generic T".to_string(),
                components: None,
                type_parameters: None,
            },
            TypeDeclaration {
                type_id: 2,
                type_field: "enum SomeEnum".to_string(),
                components: Some(vec![TypeApplication {
                    name: "variant".to_string(),
                    type_id: 1,
                    type_arguments: None,
                }]),
                type_parameters: Some(vec![1]),
            },
            TypeDeclaration {
                type_id: 3,
                type_field: "u8".to_string(),
                components: None,
                type_parameters: None,
            },
        ];

        let type_application = TypeApplication {
            name: "arg".to_string(),
            type_id: 2,
            type_arguments: Some(vec![TypeApplication {
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
                variants: EnumVariants::new(vec![ParamType::U8])?,
                generics: vec![ParamType::U8]
            }
        );

        Ok(())
    }

    #[test]
    fn handles_tuples() -> Result<(), Error> {
        // given
        let declarations = [
            TypeDeclaration {
                type_id: 1,
                type_field: "(_, _)".to_string(),
                components: Some(vec![
                    TypeApplication {
                        name: "__tuple_element".to_string(),
                        type_id: 3,
                        type_arguments: None,
                    },
                    TypeApplication {
                        name: "__tuple_element".to_string(),
                        type_id: 2,
                        type_arguments: None,
                    },
                ]),
                type_parameters: None,
            },
            TypeDeclaration {
                type_id: 2,
                type_field: "str[15]".to_string(),
                components: None,
                type_parameters: None,
            },
            TypeDeclaration {
                type_id: 3,
                type_field: "u8".to_string(),
                components: None,
                type_parameters: None,
            },
        ];

        let type_application = TypeApplication {
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
            ParamType::Tuple(vec![ParamType::U8, ParamType::String(15)])
        );

        Ok(())
    }

    #[test]
    fn ultimate_example() -> Result<(), Error> {
        // given
        let declarations = [
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
                        type_id: 23,
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
                        type_id: 21,
                        type_arguments: Some(vec![TypeApplication {
                            name: "".to_string(),
                            type_id: 20,
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
                        type_id: 24,
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
                    type_id: 22,
                    type_arguments: Some(vec![TypeApplication {
                        name: "".to_string(),
                        type_id: 19,
                        type_arguments: Some(vec![TypeApplication {
                            name: "".to_string(),
                            type_id: 9,
                            type_arguments: None,
                        }]),
                    }]),
                }]),
                type_parameters: Some(vec![9]),
            },
            TypeDeclaration {
                type_id: 18,
                type_field: "struct RawVec".to_string(),
                components: Some(vec![
                    TypeApplication {
                        name: "ptr".to_string(),
                        type_id: 24,
                        type_arguments: None,
                    },
                    TypeApplication {
                        name: "cap".to_string(),
                        type_id: 24,
                        type_arguments: None,
                    },
                ]),
                type_parameters: Some(vec![13]),
            },
            TypeDeclaration {
                type_id: 19,
                type_field: "struct SimpleGeneric".to_string(),
                components: Some(vec![TypeApplication {
                    name: "single_generic_param".to_string(),
                    type_id: 13,
                    type_arguments: None,
                }]),
                type_parameters: Some(vec![13]),
            },
            TypeDeclaration {
                type_id: 20,
                type_field: "struct StructWArrayGeneric".to_string(),
                components: Some(vec![TypeApplication {
                    name: "a".to_string(),
                    type_id: 6,
                    type_arguments: None,
                }]),
                type_parameters: Some(vec![10]),
            },
            TypeDeclaration {
                type_id: 21,
                type_field: "struct StructWTupleGeneric".to_string(),
                components: Some(vec![TypeApplication {
                    name: "a".to_string(),
                    type_id: 1,
                    type_arguments: None,
                }]),
                type_parameters: Some(vec![11]),
            },
            TypeDeclaration {
                type_id: 22,
                type_field: "struct Vec".to_string(),
                components: Some(vec![
                    TypeApplication {
                        name: "buf".to_string(),
                        type_id: 18,
                        type_arguments: Some(vec![TypeApplication {
                            name: "".to_string(),
                            type_id: 13,
                            type_arguments: None,
                        }]),
                    },
                    TypeApplication {
                        name: "len".to_string(),
                        type_id: 24,
                        type_arguments: None,
                    },
                ]),
                type_parameters: Some(vec![13]),
            },
            TypeDeclaration {
                type_id: 23,
                type_field: "u32".to_string(),
                components: None,
                type_parameters: None,
            },
            TypeDeclaration {
                type_id: 24,
                type_field: "u64".to_string(),
                components: None,
                type_parameters: None,
            },
        ];

        let type_application = TypeApplication {
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
        };

        let type_lookup = declarations
            .into_iter()
            .map(|decl| (decl.type_id, decl))
            .collect::<HashMap<_, _>>();

        // when
        let result = ParamType::try_from_type_application(&type_application, &type_lookup)?;

        // then
        let pass_the_generic_on = ParamType::Struct {
            fields: vec![ParamType::Vector(Box::new(ParamType::Struct {
                fields: vec![ParamType::String(2)],
                generics: vec![ParamType::String(2)],
            }))],
            generics: vec![ParamType::String(2)],
        };
        let struct_w_array_generic = ParamType::Struct {
            fields: vec![ParamType::Array(Box::new(pass_the_generic_on.clone()), 2)],
            generics: vec![pass_the_generic_on],
        };
        let struct_w_tuple_generic = ParamType::Struct {
            fields: vec![ParamType::Tuple(vec![
                struct_w_array_generic.clone(),
                struct_w_array_generic.clone(),
            ])],
            generics: vec![struct_w_array_generic],
        };

        let expected = ParamType::Struct {
            fields: vec![
                ParamType::Tuple(vec![
                    ParamType::Array(Box::new(ParamType::B256), 2),
                    ParamType::String(2),
                ]),
                ParamType::Tuple(vec![
                    ParamType::Array(
                        Box::new(ParamType::Enum {
                            variants: EnumVariants::new(vec![
                                ParamType::U64,
                                struct_w_tuple_generic.clone(),
                            ])?,
                            generics: vec![struct_w_tuple_generic],
                        }),
                        1,
                    ),
                    ParamType::U32,
                ]),
            ],
            generics: vec![ParamType::String(2), ParamType::B256],
        };

        assert_eq!(result, expected);

        Ok(())
    }
}
