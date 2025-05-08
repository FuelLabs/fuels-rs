use std::fmt::{Display, Formatter};

use fuel_abi_types::{
    abi::full_program::FullTypeApplication,
    utils::{self, extract_array_len, extract_generic_name, extract_str_len, has_tuple_format},
};
use proc_macro2::{Ident, TokenStream};
use quote::{ToTokens, quote};

use crate::{
    error::{Result, error},
    program_bindings::utils::sdk_provided_custom_types_lookup,
    utils::TypePath,
};

#[derive(Debug, Clone, PartialEq)]
pub enum GenericType {
    Named(Ident),
    Constant(usize),
}

impl ToTokens for GenericType {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let stream = match self {
            GenericType::Named(ident) => ident.to_token_stream(),
            GenericType::Constant(constant) => constant.to_token_stream(),
        };

        tokens.extend(stream);
    }
}

/// Represents a Rust type alongside its generic parameters. For when you want to reference an ABI
/// type in Rust code since [`ResolvedType`] can be converted into a [`TokenStream`] via
/// `resolved_type.to_token_stream()`.
#[derive(Debug, Clone)]
pub enum ResolvedType {
    Unit,
    Primitive(TypePath),
    StructOrEnum {
        path: TypePath,
        generics: Vec<ResolvedType>,
    },
    Array(Box<ResolvedType>, usize),
    Tuple(Vec<ResolvedType>),
    Generic(GenericType),
}

impl ResolvedType {
    pub fn generics(&self) -> Vec<GenericType> {
        match self {
            ResolvedType::StructOrEnum {
                generics: elements, ..
            }
            | ResolvedType::Tuple(elements) => {
                elements.iter().flat_map(|el| el.generics()).collect()
            }
            ResolvedType::Array(el, _) => el.generics(),
            ResolvedType::Generic(inner) => vec![inner.clone()],
            _ => vec![],
        }
    }
}

impl ToTokens for ResolvedType {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let tokenized = match self {
            ResolvedType::Unit => quote! {()},
            ResolvedType::Primitive(path) => path.into_token_stream(),
            ResolvedType::StructOrEnum { path, generics } => {
                if generics.is_empty() {
                    path.to_token_stream()
                } else {
                    quote! { #path<#(#generics),*>}
                }
            }
            ResolvedType::Array(el, count) => quote! { [#el; #count]},
            ResolvedType::Tuple(elements) => {
                // it is important to leave a trailing comma because a tuple with
                // one element is written as (element,) not (element) which is
                // resolved to just element
                quote! { (#(#elements,)*) }
            }
            ResolvedType::Generic(generic_type) => generic_type.into_token_stream(),
        };

        tokens.extend(tokenized)
    }
}

impl Display for ResolvedType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_token_stream())
    }
}

/// Used to resolve [`FullTypeApplication`]s into [`ResolvedType`]s
pub(crate) struct TypeResolver {
    /// The mod in which the produced [`ResolvedType`]s are going to end up in.
    current_mod: TypePath,
}

impl Default for TypeResolver {
    fn default() -> Self {
        TypeResolver::new(Default::default())
    }
}

impl TypeResolver {
    pub(crate) fn new(current_mod: TypePath) -> Self {
        Self { current_mod }
    }

    pub(crate) fn resolve(&self, type_application: &FullTypeApplication) -> Result<ResolvedType> {
        let resolvers = [
            Self::try_as_primitive_type,
            Self::try_as_bits256,
            Self::try_as_generic,
            Self::try_as_array,
            Self::try_as_sized_ascii_string,
            Self::try_as_ascii_string,
            Self::try_as_tuple,
            Self::try_as_raw_slice,
            Self::try_as_custom_type,
        ];

        for resolver in resolvers {
            if let Some(resolved) = resolver(self, type_application)? {
                return Ok(resolved);
            }
        }

        let type_field = &type_application.type_decl.type_field;
        Err(error!("could not resolve '{type_field}' to any known type"))
    }

    fn resolve_multiple(
        &self,
        type_applications: &[FullTypeApplication],
    ) -> Result<Vec<ResolvedType>> {
        type_applications
            .iter()
            .map(|type_application| self.resolve(type_application))
            .collect()
    }

    fn try_as_generic(
        &self,
        type_application: &FullTypeApplication,
    ) -> Result<Option<ResolvedType>> {
        let Some(name) = extract_generic_name(&type_application.type_decl.type_field) else {
            return Ok(None);
        };

        let ident = utils::safe_ident(&name);
        Ok(Some(ResolvedType::Generic(GenericType::Named(ident))))
    }

    fn try_as_array(&self, type_application: &FullTypeApplication) -> Result<Option<ResolvedType>> {
        let type_decl = &type_application.type_decl;
        let Some(len) = extract_array_len(&type_decl.type_field) else {
            return Ok(None);
        };

        let components = self.resolve_multiple(&type_decl.components)?;
        let type_inside = match components.as_slice() {
            [single_type] => single_type,
            other => {
                return Err(error!(
                    "array must have only one component. Actual components: {other:?}"
                ));
            }
        };

        Ok(Some(ResolvedType::Array(
            Box::new(type_inside.clone()),
            len,
        )))
    }

    fn try_as_sized_ascii_string(
        &self,
        type_application: &FullTypeApplication,
    ) -> Result<Option<ResolvedType>> {
        let Some(len) = extract_str_len(&type_application.type_decl.type_field) else {
            return Ok(None);
        };

        let path =
            TypePath::new("::fuels::types::SizedAsciiString").expect("this is a valid TypePath");
        Ok(Some(ResolvedType::StructOrEnum {
            path,
            generics: vec![ResolvedType::Generic(GenericType::Constant(len))],
        }))
    }

    fn try_as_ascii_string(
        &self,
        type_application: &FullTypeApplication,
    ) -> Result<Option<ResolvedType>> {
        let maybe_resolved = (type_application.type_decl.type_field == "str").then(|| {
            let path =
                TypePath::new("::fuels::types::AsciiString").expect("this is a valid TypePath");
            ResolvedType::StructOrEnum {
                path,
                generics: vec![],
            }
        });

        Ok(maybe_resolved)
    }

    fn try_as_tuple(&self, type_application: &FullTypeApplication) -> Result<Option<ResolvedType>> {
        let type_decl = &type_application.type_decl;
        if !has_tuple_format(&type_decl.type_field) {
            return Ok(None);
        }
        let inner_types = self.resolve_multiple(&type_decl.components)?;

        Ok(Some(ResolvedType::Tuple(inner_types)))
    }

    fn try_as_primitive_type(
        &self,
        type_decl: &FullTypeApplication,
    ) -> Result<Option<ResolvedType>> {
        let type_field = &type_decl.type_decl.type_field;

        let maybe_resolved = match type_field.as_str() {
            "()" => Some(ResolvedType::Unit),
            "bool" | "u8" | "u16" | "u32" | "u64" => {
                let path = format!("::core::primitive::{type_field}");
                let type_path = TypePath::new(path).expect("to be a valid path");

                Some(ResolvedType::Primitive(type_path))
            }
            "struct std::u128::U128" | "struct U128" => {
                let u128_path = TypePath::new("::core::primitive::u128").expect("is correct");
                Some(ResolvedType::Primitive(u128_path))
            }
            "u256" => {
                let u256_path = TypePath::new("::fuels::types::U256").expect("is correct");
                Some(ResolvedType::Primitive(u256_path))
            }
            _ => None,
        };

        Ok(maybe_resolved)
    }

    fn try_as_bits256(
        &self,
        type_application: &FullTypeApplication,
    ) -> Result<Option<ResolvedType>> {
        if type_application.type_decl.type_field != "b256" {
            return Ok(None);
        }

        let path = TypePath::new("::fuels::types::Bits256").expect("to be valid");
        Ok(Some(ResolvedType::StructOrEnum {
            path,
            generics: vec![],
        }))
    }

    fn try_as_raw_slice(
        &self,
        type_application: &FullTypeApplication,
    ) -> Result<Option<ResolvedType>> {
        if type_application.type_decl.type_field != "raw untyped slice" {
            return Ok(None);
        }

        let path = TypePath::new("::fuels::types::RawSlice").expect("this is a valid TypePath");
        Ok(Some(ResolvedType::StructOrEnum {
            path,
            generics: vec![],
        }))
    }

    fn try_as_custom_type(
        &self,
        type_application: &FullTypeApplication,
    ) -> Result<Option<ResolvedType>> {
        let type_decl = &type_application.type_decl;

        if !type_decl.is_custom_type() {
            return Ok(None);
        }

        let original_path = type_decl.custom_type_path()?;

        let used_path = sdk_provided_custom_types_lookup()
            .get(&original_path)
            .cloned()
            .unwrap_or_else(|| original_path.relative_path_from(&self.current_mod));

        let generics = self.resolve_multiple(&type_application.type_arguments)?;

        Ok(Some(ResolvedType::StructOrEnum {
            path: used_path,
            generics,
        }))
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, str::FromStr};

    use fuel_abi_types::{
        abi::{
            full_program::FullTypeDeclaration,
            unified_program::{UnifiedTypeApplication, UnifiedTypeDeclaration},
        },
        utils::ident,
    };

    use super::*;

    #[test]
    fn correctly_extracts_used_generics() {
        let resolved_type = ResolvedType::StructOrEnum {
            path: Default::default(),
            generics: vec![
                ResolvedType::Tuple(vec![ResolvedType::Array(
                    Box::new(ResolvedType::StructOrEnum {
                        path: Default::default(),
                        generics: vec![
                            ResolvedType::Generic(GenericType::Named(ident("A"))),
                            ResolvedType::Generic(GenericType::Constant(10)),
                        ],
                    }),
                    2,
                )]),
                ResolvedType::Generic(GenericType::Named(ident("B"))),
            ],
        };

        let generics = resolved_type.generics();

        assert_eq!(
            generics,
            vec![
                GenericType::Named(ident("A")),
                GenericType::Constant(10),
                GenericType::Named(ident("B"))
            ]
        )
    }

    fn test_resolve_first_type(
        expected: &str,
        type_declarations: &[UnifiedTypeDeclaration],
    ) -> Result<()> {
        let types = type_declarations
            .iter()
            .map(|td| (td.type_id, td.clone()))
            .collect::<HashMap<_, _>>();
        let type_application = UnifiedTypeApplication {
            type_id: type_declarations[0].type_id,
            ..Default::default()
        };

        let application = FullTypeApplication::from_counterpart(&type_application, &types);
        let resolved_type = TypeResolver::default()
            .resolve(&application)
            .map_err(|e| e.combine(error!("failed to resolve {:?}", type_application)))?;
        let actual = resolved_type.to_token_stream().to_string();

        let expected = TokenStream::from_str(expected).unwrap().to_string();
        assert_eq!(actual, expected);

        Ok(())
    }

    fn test_resolve_primitive_type(type_field: &str, expected: &str) -> Result<()> {
        test_resolve_first_type(
            expected,
            &[UnifiedTypeDeclaration {
                type_id: 0,
                type_field: type_field.to_string(),
                ..Default::default()
            }],
        )
    }

    #[test]
    fn test_resolve_u8() -> Result<()> {
        test_resolve_primitive_type("u8", "::core::primitive::u8")
    }

    #[test]
    fn test_resolve_u16() -> Result<()> {
        test_resolve_primitive_type("u16", "::core::primitive::u16")
    }

    #[test]
    fn test_resolve_u32() -> Result<()> {
        test_resolve_primitive_type("u32", "::core::primitive::u32")
    }

    #[test]
    fn test_resolve_u64() -> Result<()> {
        test_resolve_primitive_type("u64", "::core::primitive::u64")
    }

    #[test]
    fn test_resolve_bool() -> Result<()> {
        test_resolve_primitive_type("bool", "::core::primitive::bool")
    }

    #[test]
    fn test_resolve_b256() -> Result<()> {
        test_resolve_primitive_type("b256", "::fuels::types::Bits256")
    }

    #[test]
    fn test_resolve_unit() -> Result<()> {
        test_resolve_primitive_type("()", "()")
    }

    #[test]
    fn test_resolve_array() -> Result<()> {
        test_resolve_first_type(
            "[::core::primitive::u8 ; 3usize]",
            &[
                UnifiedTypeDeclaration {
                    type_id: 0,
                    type_field: "[u8; 3]".to_string(),
                    components: Some(vec![UnifiedTypeApplication {
                        type_id: 1,
                        ..Default::default()
                    }]),
                    ..Default::default()
                },
                UnifiedTypeDeclaration {
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
                UnifiedTypeDeclaration {
                    type_id: 0,
                    type_field: "struct std::vec::Vec".to_string(),
                    components: Some(vec![
                        UnifiedTypeApplication {
                            name: "buf".to_string(),
                            type_id: 2,
                            type_arguments: Some(vec![UnifiedTypeApplication {
                                type_id: 1,
                                ..Default::default()
                            }]),
                            error_message: None,
                        },
                        UnifiedTypeApplication {
                            name: "len".to_string(),
                            type_id: 3,
                            ..Default::default()
                        },
                    ]),
                    type_parameters: Some(vec![1]),
                },
                UnifiedTypeDeclaration {
                    type_id: 1,
                    type_field: "generic T".to_string(),
                    ..Default::default()
                },
                UnifiedTypeDeclaration {
                    type_id: 2,
                    type_field: "raw untyped ptr".to_string(),
                    ..Default::default()
                },
                UnifiedTypeDeclaration {
                    type_id: 3,
                    type_field: "struct std::vec::RawVec".to_string(),
                    components: Some(vec![
                        UnifiedTypeApplication {
                            name: "ptr".to_string(),
                            type_id: 2,
                            ..Default::default()
                        },
                        UnifiedTypeApplication {
                            name: "cap".to_string(),
                            type_id: 4,
                            ..Default::default()
                        },
                    ]),
                    type_parameters: Some(vec![1]),
                },
                UnifiedTypeDeclaration {
                    type_id: 4,
                    type_field: "u64".to_string(),
                    ..Default::default()
                },
                UnifiedTypeDeclaration {
                    type_id: 5,
                    type_field: "u8".to_string(),
                    ..Default::default()
                },
            ],
        )
    }

    #[test]
    fn test_resolve_bytes() -> Result<()> {
        test_resolve_first_type(
            ":: fuels :: types :: Bytes",
            &[
                UnifiedTypeDeclaration {
                    type_id: 0,
                    type_field: "struct String".to_string(),
                    components: Some(vec![UnifiedTypeApplication {
                        name: "bytes".to_string(),
                        type_id: 1,
                        ..Default::default()
                    }]),
                    ..Default::default()
                },
                UnifiedTypeDeclaration {
                    type_id: 0,
                    type_field: "struct std::bytes::Bytes".to_string(),
                    components: Some(vec![
                        UnifiedTypeApplication {
                            name: "buf".to_string(),
                            type_id: 1,
                            ..Default::default()
                        },
                        UnifiedTypeApplication {
                            name: "len".to_string(),
                            type_id: 3,
                            ..Default::default()
                        },
                    ]),
                    ..Default::default()
                },
                UnifiedTypeDeclaration {
                    type_id: 1,
                    type_field: "struct std::bytes::RawBytes".to_string(),
                    components: Some(vec![
                        UnifiedTypeApplication {
                            name: "ptr".to_string(),
                            type_id: 2,
                            ..Default::default()
                        },
                        UnifiedTypeApplication {
                            name: "cap".to_string(),
                            type_id: 3,
                            ..Default::default()
                        },
                    ]),
                    ..Default::default()
                },
                UnifiedTypeDeclaration {
                    type_id: 2,
                    type_field: "raw untyped ptr".to_string(),
                    ..Default::default()
                },
                UnifiedTypeDeclaration {
                    type_id: 3,
                    type_field: "u64".to_string(),
                    ..Default::default()
                },
            ],
        )
    }

    #[test]
    fn test_resolve_std_string() -> Result<()> {
        test_resolve_first_type(
            ":: std :: string :: String",
            &[
                UnifiedTypeDeclaration {
                    type_id: 0,
                    type_field: "struct std::string::String".to_string(),
                    components: Some(vec![UnifiedTypeApplication {
                        name: "bytes".to_string(),
                        type_id: 1,
                        ..Default::default()
                    }]),
                    ..Default::default()
                },
                UnifiedTypeDeclaration {
                    type_id: 1,
                    type_field: "struct std::bytes::Bytes".to_string(),
                    components: Some(vec![
                        UnifiedTypeApplication {
                            name: "buf".to_string(),
                            type_id: 2,
                            ..Default::default()
                        },
                        UnifiedTypeApplication {
                            name: "len".to_string(),
                            type_id: 4,
                            ..Default::default()
                        },
                    ]),
                    ..Default::default()
                },
                UnifiedTypeDeclaration {
                    type_id: 2,
                    type_field: "struct std::bytes::RawBytes".to_string(),
                    components: Some(vec![
                        UnifiedTypeApplication {
                            name: "ptr".to_string(),
                            type_id: 3,
                            ..Default::default()
                        },
                        UnifiedTypeApplication {
                            name: "cap".to_string(),
                            type_id: 4,
                            ..Default::default()
                        },
                    ]),
                    ..Default::default()
                },
                UnifiedTypeDeclaration {
                    type_id: 3,
                    type_field: "raw untyped ptr".to_string(),
                    ..Default::default()
                },
                UnifiedTypeDeclaration {
                    type_id: 4,
                    type_field: "u64".to_string(),
                    ..Default::default()
                },
            ],
        )
    }

    #[test]
    fn test_resolve_static_str() -> Result<()> {
        test_resolve_primitive_type("str[3]", ":: fuels :: types :: SizedAsciiString < 3usize >")
    }

    #[test]
    fn test_resolve_struct() -> Result<()> {
        test_resolve_first_type(
            "self :: SomeStruct",
            &[
                UnifiedTypeDeclaration {
                    type_id: 0,
                    type_field: "struct SomeStruct".to_string(),
                    components: Some(vec![
                        UnifiedTypeApplication {
                            name: "foo".to_string(),
                            type_id: 1,
                            ..Default::default()
                        },
                        UnifiedTypeApplication {
                            name: "bar".to_string(),
                            type_id: 2,
                            ..Default::default()
                        },
                    ]),
                    type_parameters: Some(vec![1]),
                },
                UnifiedTypeDeclaration {
                    type_id: 1,
                    type_field: "generic T".to_string(),
                    ..Default::default()
                },
                UnifiedTypeDeclaration {
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
                UnifiedTypeDeclaration {
                    type_id: 0,
                    type_field: "enum SomeEnum".to_string(),
                    components: Some(vec![
                        UnifiedTypeApplication {
                            name: "foo".to_string(),
                            type_id: 1,
                            ..Default::default()
                        },
                        UnifiedTypeApplication {
                            name: "bar".to_string(),
                            type_id: 2,
                            ..Default::default()
                        },
                    ]),
                    type_parameters: Some(vec![1]),
                },
                UnifiedTypeDeclaration {
                    type_id: 1,
                    type_field: "generic T".to_string(),
                    ..Default::default()
                },
                UnifiedTypeDeclaration {
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
            "(::core::primitive::u8, ::core::primitive::u16, ::core::primitive::bool, T,)",
            &[
                UnifiedTypeDeclaration {
                    type_id: 0,
                    type_field: "(u8, u16, bool, T)".to_string(),
                    components: Some(vec![
                        UnifiedTypeApplication {
                            type_id: 1,
                            ..Default::default()
                        },
                        UnifiedTypeApplication {
                            type_id: 2,
                            ..Default::default()
                        },
                        UnifiedTypeApplication {
                            type_id: 3,
                            ..Default::default()
                        },
                        UnifiedTypeApplication {
                            type_id: 4,
                            ..Default::default()
                        },
                    ]),
                    type_parameters: Some(vec![4]),
                },
                UnifiedTypeDeclaration {
                    type_id: 1,
                    type_field: "u8".to_string(),
                    ..Default::default()
                },
                UnifiedTypeDeclaration {
                    type_id: 2,
                    type_field: "u16".to_string(),
                    ..Default::default()
                },
                UnifiedTypeDeclaration {
                    type_id: 3,
                    type_field: "bool".to_string(),
                    ..Default::default()
                },
                UnifiedTypeDeclaration {
                    type_id: 4,
                    type_field: "generic T".to_string(),
                    ..Default::default()
                },
            ],
        )
    }

    #[test]
    fn custom_types_uses_correct_path_for_sdk_provided_types() {
        let resolver = TypeResolver::default();
        for (type_path, expected_path) in sdk_provided_custom_types_lookup() {
            // given
            let type_application = given_fn_arg_of_custom_type(&type_path);

            // when
            let resolved_type = resolver.resolve(&type_application).unwrap();

            // then
            let expected_type_name = expected_path.into_token_stream();
            assert_eq!(
                resolved_type.to_token_stream().to_string(),
                expected_type_name.to_string()
            );
        }
    }

    fn given_fn_arg_of_custom_type(type_path: &TypePath) -> FullTypeApplication {
        FullTypeApplication {
            name: "some_arg".to_string(),
            type_decl: FullTypeDeclaration {
                type_field: format!("struct {type_path}"),
                components: vec![],
                type_parameters: vec![],
            },
            type_arguments: vec![],
            error_message: None,
        }
    }
}
