use std::fmt::{Display, Formatter};

use fuel_abi_types::utils::{
    extract_array_len, extract_custom_type_name, extract_generic_name, extract_str_len,
    has_tuple_format,
};
use lazy_static::lazy_static;
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use regex::Regex;

use crate::{
    error::{error, Result},
    program_bindings::{abi_types::FullTypeApplication, utils::sdk_provided_custom_types_lookup},
    utils::{safe_ident, TypePath},
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

impl ToTokens for ResolvedType {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let type_name = &self.type_name;

        let tokenized_type = if self.generic_params.is_empty() {
            type_name.clone()
        } else {
            let generic_params = self.generic_params.iter().map(ToTokens::to_token_stream);

            quote! { #type_name<#( #generic_params ),*> }
        };

        tokens.extend(tokenized_type)
    }
}
impl Display for ResolvedType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_token_stream())
    }
}

pub(crate) struct TypeResolver {
    relative_to_mod: TypePath,
}

impl TypeResolver {
    pub(crate) fn new() -> Self {
        Self {
            relative_to_mod: Default::default(),
        }
    }

    /// All custom type paths will be resolved relative from the given `type_path`. E.g. When
    /// resolving a struct containing a field of the custom type `some_lib::another_lib::AType` with
    /// `type_path` being given as `some_lib::different_lib` the resulting path to `AType` will be
    /// relative to `type_path`: `super::another_lib::AType`.
    pub(crate) fn relative_to_mod(&mut self, type_path: TypePath) -> &mut Self {
        self.relative_to_mod = type_path;
        self
    }

    /// Given a type, will recursively proceed to resolve it until it results in a
    /// `ResolvedType` which can be then be converted into a `TokenStream`. As such
    /// it can be used whenever you need the Rust type of the given
    /// `FullTypeApplication`.
    pub(crate) fn resolve(&self, type_application: &FullTypeApplication) -> Result<ResolvedType> {
        let base_type = &type_application.type_decl;

        let type_field = base_type.type_field.as_str();

        [
            Self::to_simple_type,
            Self::to_byte,
            Self::to_bits256,
            Self::to_generic,
            Self::to_array,
            Self::to_sized_ascii_string,
            Self::to_tuple,
            Self::to_raw_slice,
            Self::to_custom_type,
        ]
        .into_iter()
        .find_map(|fun| fun(self, type_application))
        .ok_or_else(|| error!("Could not resolve '{type_field}' to any known type"))
    }

    fn recursively_resolve_2(
        &self,
        type_applications: &[FullTypeApplication],
    ) -> Result<Vec<ResolvedType>> {
        type_applications
            .iter()
            .map(|type_application| self.resolve(type_application))
            .collect()
    }

    fn to_generic(&self, type_application: &FullTypeApplication) -> Option<ResolvedType> {
        let name = extract_generic_name(&type_application.type_decl.type_field)?;

        let type_name = safe_ident(&name).into_token_stream();
        Some(ResolvedType {
            type_name,
            generic_params: vec![],
        })
    }

    fn to_array(&self, type_application: &FullTypeApplication) -> Option<ResolvedType> {
        let len = extract_array_len(&type_application.type_decl.type_field)?;

        let components = self
            .recursively_resolve_2(&type_application.type_decl.components)
            .unwrap();
        let type_inside = match components.as_slice() {
            [single_type] => Ok(single_type),
            other => Err(error!(
                "Array must have only one component! Actual components: {other:?}"
            )),
        }
        .unwrap();

        Some(ResolvedType {
            type_name: quote! { [#type_inside; #len] },
            generic_params: vec![],
        })
    }

    fn to_sized_ascii_string(
        &self,
        type_application: &FullTypeApplication,
    ) -> Option<ResolvedType> {
        let len = extract_str_len(&type_application.type_decl.type_field)?;

        let generic_params = vec![ResolvedType {
            type_name: quote! {#len},
            generic_params: vec![],
        }];

        Some(ResolvedType {
            type_name: quote! { ::fuels::types::SizedAsciiString },
            generic_params,
        })
    }

    fn to_tuple(&self, type_application: &FullTypeApplication) -> Option<ResolvedType> {
        if has_tuple_format(&type_application.type_decl.type_field) {
            let inner_types = self
                .recursively_resolve_2(&type_application.type_decl.components)
                .unwrap();

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

    fn to_simple_type(&self, type_decl: &FullTypeApplication) -> Option<ResolvedType> {
        let type_field = &type_decl.type_decl.type_field;
        match type_field.as_str() {
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

    fn to_byte(&self, type_application: &FullTypeApplication) -> Option<ResolvedType> {
        if type_application.type_decl.type_field == "byte" {
            Some(ResolvedType {
                type_name: quote! {::fuels::types::Byte},
                generic_params: vec![],
            })
        } else {
            None
        }
    }

    fn to_bits256(&self, type_application: &FullTypeApplication) -> Option<ResolvedType> {
        if type_application.type_decl.type_field == "b256" {
            Some(ResolvedType {
                type_name: quote! {::fuels::types::Bits256},
                generic_params: vec![],
            })
        } else {
            None
        }
    }

    fn to_raw_slice(&self, type_application: &FullTypeApplication) -> Option<ResolvedType> {
        if type_application.type_decl.type_field == "raw untyped slice" {
            let type_name = quote! {::fuels::types::RawSlice};
            Some(ResolvedType {
                type_name,
                generic_params: vec![],
            })
        } else {
            None
        }
    }

    fn to_custom_type(&self, type_application: &FullTypeApplication) -> Option<ResolvedType> {
        let type_path = extract_custom_type_name(&type_application.type_decl.type_field)?;

        let type_path = sdk_provided_custom_types_lookup()
            .get(&type_path)
            .cloned()
            .unwrap_or_else(|| {
                TypePath::new(type_path)
                    .unwrap()
                    .relative_path_from(&self.relative_to_mod)
            });

        let generic_params = self
            .recursively_resolve_2(&type_application.type_arguments)
            .unwrap();
        Some(ResolvedType {
            type_name: type_path.into_token_stream(),
            generic_params,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use fuel_abi_types::program_abi::{TypeApplication, TypeDeclaration};

    use super::*;
    use crate::program_bindings::abi_types::FullTypeDeclaration;

    fn test_resolve_first_type(
        expected: &str,
        type_declarations: &[TypeDeclaration],
    ) -> Result<()> {
        let types = type_declarations
            .iter()
            .map(|td| (td.type_id, td.clone()))
            .collect::<HashMap<_, _>>();
        let type_application = TypeApplication {
            type_id: type_declarations[0].type_id,
            ..Default::default()
        };

        let application = FullTypeApplication::from_counterpart(&type_application, &types);
        let resolved_type = TypeResolver::new()
            .resolve(&application)
            .map_err(|e| e.combine(error!("failed to resolve {:?}", type_application)))?;
        let actual = resolved_type.to_token_stream().to_string();

        assert_eq!(actual, expected);

        Ok(())
    }

    fn test_resolve_primitive_type(type_field: &str, expected: &str) -> Result<()> {
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
    fn test_resolve_u8() -> Result<()> {
        test_resolve_primitive_type("u8", "u8")
    }

    #[test]
    fn test_resolve_u16() -> Result<()> {
        test_resolve_primitive_type("u16", "u16")
    }

    #[test]
    fn test_resolve_u32() -> Result<()> {
        test_resolve_primitive_type("u32", "u32")
    }

    #[test]
    fn test_resolve_u64() -> Result<()> {
        test_resolve_primitive_type("u64", "u64")
    }

    #[test]
    fn test_resolve_bool() -> Result<()> {
        test_resolve_primitive_type("bool", "bool")
    }

    #[test]
    fn test_resolve_byte() -> Result<()> {
        test_resolve_primitive_type("byte", ":: fuels :: types :: Byte")
    }

    #[test]
    fn test_resolve_b256() -> Result<()> {
        test_resolve_primitive_type("b256", ":: fuels :: types :: Bits256")
    }

    #[test]
    fn test_resolve_unit() -> Result<()> {
        test_resolve_primitive_type("()", "()")
    }

    #[test]
    fn test_resolve_array() -> Result<()> {
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
    fn test_resolve_vector() -> Result<()> {
        test_resolve_first_type(
            ":: std :: vec :: Vec",
            &[
                TypeDeclaration {
                    type_id: 0,
                    type_field: "struct std::vec::Vec".to_string(),
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
                    type_field: "struct std::vec::RawVec".to_string(),
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
    fn test_resolve_string() -> Result<()> {
        test_resolve_primitive_type("str[3]", ":: fuels :: types :: SizedAsciiString < 3usize >")
    }

    #[test]
    fn test_resolve_struct() -> Result<()> {
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
    fn test_resolve_enum() -> Result<()> {
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
    fn test_resolve_tuple() -> Result<()> {
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
        for (type_name, expected_path) in sdk_provided_custom_types_lookup() {
            let type_application = FullTypeApplication {
                name: "".to_string(),
                type_decl: FullTypeDeclaration {
                    type_field: format!("struct {type_name}"),
                    components: vec![],
                    type_parameters: vec![],
                },
                type_arguments: vec![],
            };
            let resolved_type = TypeResolver::new().resolve(&type_application).unwrap();

            let expected_type_name = expected_path.into_token_stream();
            assert_eq!(
                resolved_type.type_name.to_string(),
                expected_type_name.to_string()
            );
        }
    }
}
