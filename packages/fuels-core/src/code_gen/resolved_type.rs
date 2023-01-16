use std::collections::HashSet;
use std::fmt::{Display, Formatter};

use lazy_static::lazy_static;
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use regex::Regex;

use fuels_types::{
    errors::Error,
    utils::custom_type_name,
    utils::{extract_array_len, extract_generic_name, extract_str_len, has_tuple_format},
};

use crate::{
    code_gen::{
        abi_types::{FullTypeApplication, FullTypeDeclaration},
        type_path::TypePath,
        utils::get_sdk_provided_types,
    },
    utils::{ident, safe_ident},
};

// Represents a type alongside its generic parameters. Can be converted into a
// `TokenStream` via `.into()`.
#[derive(Debug, Clone)]
pub struct ResolvedType {
    pub type_name: TokenStream,
    pub generic_params: Vec<ResolvedType>,
}

impl ResolvedType {
    pub fn is_unit(&self) -> bool {
        self.type_name.to_string() == "()"
    }
    // Used to prevent returning vectors until we get
    // the compiler support for it.
    #[must_use]
    pub fn uses_vectors(&self) -> bool {
        lazy_static! {
            static ref RE: Regex = Regex::new(r"\bVec\b").unwrap();
        }
        RE.is_match(&self.type_name.to_string())
            || self.generic_params.iter().any(ResolvedType::uses_vectors)
    }
}

impl Display for ResolvedType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", TokenStream::from(self.clone()))
    }
}

impl From<&ResolvedType> for TokenStream {
    fn from(resolved_type: &ResolvedType) -> Self {
        let type_name = &resolved_type.type_name;
        if resolved_type.generic_params.is_empty() {
            return quote! { #type_name };
        }

        let generic_params = resolved_type.generic_params.iter().map(TokenStream::from);

        quote! { #type_name<#( #generic_params ),*> }
    }
}

impl From<ResolvedType> for TokenStream {
    fn from(resolved_type: ResolvedType) -> Self {
        (&resolved_type).into()
    }
}

/// Given a type, will recursively proceed to resolve it until it results in a
/// `ResolvedType` which can be then be converted into a `TokenStream`. As such
/// it can be used whenever you need the Rust type of the given
/// `TypeApplication`.
pub(crate) fn resolve_type(
    type_application: &FullTypeApplication,
    shared_types: &HashSet<FullTypeDeclaration>,
) -> Result<ResolvedType, Error> {
    let recursively_resolve = |type_applications: &Vec<FullTypeApplication>| {
        type_applications
            .iter()
            .map(|type_application| resolve_type(type_application, shared_types))
            .collect::<Result<Vec<_>, _>>()
            .expect("Failed to resolve types")
    };

    let base_type = &type_application.type_decl;

    let type_field = base_type.type_field.as_str();

    [
        to_simple_type,
        to_byte,
        to_bits256,
        to_generic,
        to_array,
        to_sized_ascii_string,
        to_tuple,
        to_custom_type,
    ]
    .into_iter()
    .find_map(|fun| {
        let is_shared = shared_types.contains(base_type);
        fun(
            type_field,
            move || recursively_resolve(&base_type.components),
            move || recursively_resolve(&type_application.type_arguments),
            is_shared,
        )
    })
    .ok_or_else(|| Error::InvalidType(format!("Could not resolve {type_field} to any known type")))
}

fn to_generic(
    type_field: &str,
    _: impl Fn() -> Vec<ResolvedType>,
    _: impl Fn() -> Vec<ResolvedType>,
    _: bool,
) -> Option<ResolvedType> {
    let name = extract_generic_name(type_field)?;

    let type_name = safe_ident(&name).into_token_stream();
    Some(ResolvedType {
        type_name,
        generic_params: vec![],
    })
}

fn to_array(
    type_field: &str,
    components_supplier: impl Fn() -> Vec<ResolvedType>,
    _: impl Fn() -> Vec<ResolvedType>,
    _: bool,
) -> Option<ResolvedType> {
    let len = extract_array_len(type_field)?;

    let type_inside: TokenStream = match components_supplier().as_slice() {
        [single_type] => Ok(single_type.into()),
        other => Err(Error::InvalidData(format!(
            "Array must have only one component! Actual components: {other:?}"
        ))),
    }
    .unwrap();

    Some(ResolvedType {
        type_name: quote! { [#type_inside; #len] },
        generic_params: vec![],
    })
}

fn to_sized_ascii_string(
    type_field: &str,
    _: impl Fn() -> Vec<ResolvedType>,
    _: impl Fn() -> Vec<ResolvedType>,
    _: bool,
) -> Option<ResolvedType> {
    let len = extract_str_len(type_field)?;

    let generic_params = vec![ResolvedType {
        type_name: quote! {#len},
        generic_params: vec![],
    }];

    Some(ResolvedType {
        type_name: quote! { ::fuels::core::types::SizedAsciiString },
        generic_params,
    })
}

fn to_tuple(
    type_field: &str,
    components_supplier: impl Fn() -> Vec<ResolvedType>,
    _: impl Fn() -> Vec<ResolvedType>,
    _: bool,
) -> Option<ResolvedType> {
    if has_tuple_format(type_field) {
        let inner_types = components_supplier().into_iter().map(TokenStream::from);

        // it is important to leave a trailing comma because a tuple with
        // one element is written as (element,) not (element) which is
        // resolved to just element
        Some(ResolvedType {
            type_name: quote! {(#(#inner_types,)*)},
            generic_params: vec![],
        })
    } else {
        None
    }
}

fn to_simple_type(
    type_field: &str,
    _: impl Fn() -> Vec<ResolvedType>,
    _: impl Fn() -> Vec<ResolvedType>,
    _: bool,
) -> Option<ResolvedType> {
    match type_field {
        "u8" | "u16" | "u32" | "u64" | "bool" | "()" => {
            let type_name = type_field
                .parse()
                .expect("Couldn't resolve primitive type. Cannot happen!");

            Some(ResolvedType {
                type_name,
                generic_params: vec![],
            })
        }
        _ => None,
    }
}

fn to_byte(
    type_field: &str,
    _: impl Fn() -> Vec<ResolvedType>,
    _: impl Fn() -> Vec<ResolvedType>,
    _: bool,
) -> Option<ResolvedType> {
    if type_field == "byte" {
        Some(ResolvedType {
            type_name: quote! {::fuels::core::types::Byte},
            generic_params: vec![],
        })
    } else {
        None
    }
}
fn to_bits256(
    type_field: &str,
    _: impl Fn() -> Vec<ResolvedType>,
    _: impl Fn() -> Vec<ResolvedType>,
    _: bool,
) -> Option<ResolvedType> {
    if type_field == "b256" {
        Some(ResolvedType {
            type_name: quote! {::fuels::core::types::Bits256},
            generic_params: vec![],
        })
    } else {
        None
    }
}

fn to_custom_type(
    type_field: &str,
    _: impl Fn() -> Vec<ResolvedType>,
    type_arguments_supplier: impl Fn() -> Vec<ResolvedType>,
    is_shared: bool,
) -> Option<ResolvedType> {
    let type_name = custom_type_name(type_field).ok()?;

    let type_path = get_sdk_provided_types()
        .into_iter()
        .find(|provided_type| provided_type.type_name() == type_name)
        .unwrap_or_else(|| {
            let custom_type_name = ident(&type_name);
            let path_str = if is_shared {
                format!("super::shared_types::{custom_type_name}")
            } else {
                format!("self::{custom_type_name}")
            };
            TypePath::new(&path_str).expect("Known to be well formed")
        });

    Some(ResolvedType {
        type_name: type_path.into(),
        generic_params: type_arguments_supplier(),
    })
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use anyhow::Context;

    use fuel_abi_types::program_abi::{TypeApplication, TypeDeclaration};

    use super::*;

    fn test_resolve_first_type(
        expected: &str,
        type_declarations: &[TypeDeclaration],
    ) -> anyhow::Result<()> {
        let types = type_declarations
            .iter()
            .map(|td| (td.type_id, td.clone()))
            .collect::<HashMap<_, _>>();
        let type_application = TypeApplication {
            type_id: type_declarations[0].type_id,
            ..Default::default()
        };

        let application = FullTypeApplication::from_counterpart(&type_application, &types);
        let resolved_type = resolve_type(&application, &HashSet::default())
            .with_context(|| format!("failed to resolve {:?}", &type_application))?;
        let actual = TokenStream::from(&resolved_type).to_string();

        assert_eq!(actual, expected);

        Ok(())
    }

    fn test_resolve_primitive_type(type_field: &str, expected: &str) -> anyhow::Result<()> {
        test_resolve_first_type(
            expected,
            &[TypeDeclaration {
                type_id: 0,
                type_field: type_field.to_string(),
                ..Default::default()
            }],
        )
    }

    #[test]
    fn test_resolve_u8() -> anyhow::Result<()> {
        test_resolve_primitive_type("u8", "u8")
    }

    #[test]
    fn test_resolve_u16() -> anyhow::Result<()> {
        test_resolve_primitive_type("u16", "u16")
    }

    #[test]
    fn test_resolve_u32() -> anyhow::Result<()> {
        test_resolve_primitive_type("u32", "u32")
    }

    #[test]
    fn test_resolve_u64() -> anyhow::Result<()> {
        test_resolve_primitive_type("u64", "u64")
    }

    #[test]
    fn test_resolve_bool() -> anyhow::Result<()> {
        test_resolve_primitive_type("bool", "bool")
    }

    #[test]
    fn test_resolve_byte() -> anyhow::Result<()> {
        test_resolve_primitive_type("byte", ":: fuels :: core :: types :: Byte")
    }

    #[test]
    fn test_resolve_b256() -> anyhow::Result<()> {
        test_resolve_primitive_type("b256", ":: fuels :: core :: types :: Bits256")
    }

    #[test]
    fn test_resolve_unit() -> anyhow::Result<()> {
        test_resolve_primitive_type("()", "()")
    }

    #[test]
    fn test_resolve_array() -> anyhow::Result<()> {
        test_resolve_first_type(
            "[u8 ; 3usize]",
            &[
                TypeDeclaration {
                    type_id: 0,
                    type_field: "[u8; 3]".to_string(),
                    components: Some(vec![TypeApplication {
                        type_id: 1,
                        ..Default::default()
                    }]),
                    ..Default::default()
                },
                TypeDeclaration {
                    type_id: 1,
                    type_field: "u8".to_string(),
                    ..Default::default()
                },
            ],
        )
    }

    #[test]
    fn test_resolve_vector() -> anyhow::Result<()> {
        test_resolve_first_type(
            ":: std :: vec :: Vec",
            &[
                TypeDeclaration {
                    type_id: 0,
                    type_field: "struct Vec".to_string(),
                    components: Some(vec![
                        TypeApplication {
                            name: "buf".to_string(),
                            type_id: 2,
                            type_arguments: Some(vec![TypeApplication {
                                type_id: 1,
                                ..Default::default()
                            }]),
                        },
                        TypeApplication {
                            name: "len".to_string(),
                            type_id: 3,
                            ..Default::default()
                        },
                    ]),
                    type_parameters: Some(vec![1]),
                },
                TypeDeclaration {
                    type_id: 1,
                    type_field: "generic T".to_string(),
                    ..Default::default()
                },
                TypeDeclaration {
                    type_id: 2,
                    type_field: "raw untyped ptr".to_string(),
                    ..Default::default()
                },
                TypeDeclaration {
                    type_id: 3,
                    type_field: "struct RawVec".to_string(),
                    components: Some(vec![
                        TypeApplication {
                            name: "ptr".to_string(),
                            type_id: 2,
                            ..Default::default()
                        },
                        TypeApplication {
                            name: "cap".to_string(),
                            type_id: 4,
                            ..Default::default()
                        },
                    ]),
                    type_parameters: Some(vec![1]),
                },
                TypeDeclaration {
                    type_id: 4,
                    type_field: "u64".to_string(),
                    ..Default::default()
                },
                TypeDeclaration {
                    type_id: 5,
                    type_field: "u8".to_string(),
                    ..Default::default()
                },
            ],
        )
    }

    #[test]
    fn test_resolve_string() -> anyhow::Result<()> {
        test_resolve_primitive_type(
            "str[3]",
            ":: fuels :: core :: types :: SizedAsciiString < 3usize >",
        )
    }

    #[test]
    fn test_resolve_struct() -> anyhow::Result<()> {
        test_resolve_first_type(
            "self :: SomeStruct",
            &[
                TypeDeclaration {
                    type_id: 0,
                    type_field: "struct SomeStruct".to_string(),
                    components: Some(vec![
                        TypeApplication {
                            name: "foo".to_string(),
                            type_id: 1,
                            ..Default::default()
                        },
                        TypeApplication {
                            name: "bar".to_string(),
                            type_id: 2,
                            ..Default::default()
                        },
                    ]),
                    type_parameters: Some(vec![1]),
                },
                TypeDeclaration {
                    type_id: 1,
                    type_field: "generic T".to_string(),
                    ..Default::default()
                },
                TypeDeclaration {
                    type_id: 2,
                    type_field: "u8".to_string(),
                    ..Default::default()
                },
            ],
        )
    }

    #[test]
    fn test_resolve_enum() -> anyhow::Result<()> {
        test_resolve_first_type(
            "self :: SomeEnum",
            &[
                TypeDeclaration {
                    type_id: 0,
                    type_field: "enum SomeEnum".to_string(),
                    components: Some(vec![
                        TypeApplication {
                            name: "foo".to_string(),
                            type_id: 1,
                            ..Default::default()
                        },
                        TypeApplication {
                            name: "bar".to_string(),
                            type_id: 2,
                            ..Default::default()
                        },
                    ]),
                    type_parameters: Some(vec![1]),
                },
                TypeDeclaration {
                    type_id: 1,
                    type_field: "generic T".to_string(),
                    ..Default::default()
                },
                TypeDeclaration {
                    type_id: 2,
                    type_field: "u8".to_string(),
                    ..Default::default()
                },
            ],
        )
    }

    #[test]
    fn test_resolve_tuple() -> anyhow::Result<()> {
        test_resolve_first_type(
            "(u8 , u16 , bool , T ,)",
            &[
                TypeDeclaration {
                    type_id: 0,
                    type_field: "(u8, u16, bool, T)".to_string(),
                    components: Some(vec![
                        TypeApplication {
                            type_id: 1,
                            ..Default::default()
                        },
                        TypeApplication {
                            type_id: 2,
                            ..Default::default()
                        },
                        TypeApplication {
                            type_id: 3,
                            ..Default::default()
                        },
                        TypeApplication {
                            type_id: 4,
                            ..Default::default()
                        },
                    ]),
                    type_parameters: Some(vec![4]),
                },
                TypeDeclaration {
                    type_id: 1,
                    type_field: "u8".to_string(),
                    ..Default::default()
                },
                TypeDeclaration {
                    type_id: 2,
                    type_field: "u16".to_string(),
                    ..Default::default()
                },
                TypeDeclaration {
                    type_id: 3,
                    type_field: "bool".to_string(),
                    ..Default::default()
                },
                TypeDeclaration {
                    type_id: 4,
                    type_field: "generic T".to_string(),
                    ..Default::default()
                },
            ],
        )
    }

    #[test]
    fn custom_types_uses_correct_path_for_sdk_provided_types() {
        let provided_type_names = get_sdk_provided_types()
            .into_iter()
            .map(|type_path| (type_path.type_name().to_string(), type_path))
            .collect::<HashMap<_, _>>();

        for (type_name, expected_path) in provided_type_names {
            let resolved_type =
                to_custom_type(&format!("struct {type_name}"), Vec::new, Vec::new, false)
                    .expect("Should have succeeded.");

            let expected_type_name: TokenStream = expected_path.into();
            assert_eq!(
                resolved_type.type_name.to_string(),
                expected_type_name.to_string()
            );
        }
    }
    #[test]
    fn handles_shared_types() {
        let resolved_type =
            to_custom_type("struct SomeStruct", Vec::new, Vec::new, true).expect("should succeed");

        assert_eq!(
            resolved_type.type_name.to_string(),
            "super :: shared_types :: SomeStruct"
        )
    }
}
