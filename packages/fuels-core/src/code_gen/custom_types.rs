use std::collections::HashSet;

use fuels_types::{errors::Error, utils::custom_type_name};
use itertools::Itertools;

use crate::code_gen::{
    abi_types::FullTypeDeclaration,
    custom_types::{enums::expand_custom_enum, structs::expand_custom_struct},
    generated_code::GeneratedCode,
    utils::get_sdk_provided_types,
};

mod enums;
mod structs;
mod utils;

/// Generates Rust code for each type inside `types` if:
/// * the type is not present inside `shared_types`, and
/// * if it should be generated (see: [`should_skip_codegen`], and
/// * if it is a struct or an enum.
///
///
/// # Arguments
///
/// * `types`: Types you wish to generate Rust code for.
/// * `shared_types`: Types that are shared between multiple
///                   contracts/scripts/predicates and thus generated elsewhere.
pub(crate) fn generate_types<T: IntoIterator<Item = FullTypeDeclaration>>(
    types: T,
    shared_types: &HashSet<FullTypeDeclaration>,
) -> Result<GeneratedCode, Error> {
    HashSet::from_iter(types)
        .difference(shared_types)
        .filter(|ttype| !should_skip_codegen(&ttype.type_field))
        .filter_map(|ttype| {
            if ttype.is_struct_type() {
                Some(expand_custom_struct(ttype, shared_types))
            } else if ttype.is_enum_type() {
                Some(expand_custom_enum(ttype, shared_types))
            } else {
                None
            }
        })
        .fold_ok(GeneratedCode::default(), |acc, generated_code| {
            acc.append(generated_code)
        })
}

// Checks whether the given type should not have code generated for it. This
// is mainly because the corresponding type in Rust already exists --
// e.g. the contract's Vec type is mapped to std::vec::Vec from the Rust
// stdlib, ContractId is a custom type implemented by fuels-rs, etc.
// Others like 'raw untyped ptr' or 'RawVec' are skipped because they are
// implementation details of the contract's Vec type and are not directly
// used in the SDK.
fn should_skip_codegen(type_field: &str) -> bool {
    let name = custom_type_name(type_field).unwrap_or_else(|_| type_field.to_string());

    is_type_sdk_provided(&name) || is_type_unused(&name)
}

fn is_type_sdk_provided(name: &str) -> bool {
    get_sdk_provided_types()
        .iter()
        .any(|type_path| type_path.type_name() == name)
}

fn is_type_unused(name: &str) -> bool {
    ["raw untyped ptr", "RawVec"].contains(&name)
}

// Doing string -> TokenStream -> string isn't pretty but gives us the opportunity to
// have a better understanding of the generated code so we consider it ok.
// To generate the expected examples, output of the functions were taken
// with code @9ca376, and formatted in-IDE using rustfmt. It should be noted that
// rustfmt added an extra `,` after the last struct/enum field, which is not added
// by the `expand_custom_*` functions, and so was removed from the expected string.
// TODO(iqdecay): append extra `,` to last enum/struct field so it is aligned with rustfmt
#[cfg(test)]
mod tests {
    use std::{
        collections::{HashMap, HashSet},
        str::FromStr,
    };

    use anyhow::anyhow;
    use fuel_abi_types::program_abi::{ProgramABI, TypeApplication, TypeDeclaration};
    use proc_macro2::TokenStream;
    use quote::quote;

    use super::*;
    use crate::code_gen::{
        abi_types::{FullTypeApplication, FullTypeDeclaration},
        type_path::TypePath,
    };

    #[test]
    fn test_expand_custom_enum() -> Result<(), Error> {
        let p = TypeDeclaration {
            type_id: 0,
            type_field: String::from("enum MatchaTea"),
            components: Some(vec![
                TypeApplication {
                    name: String::from("LongIsland"),
                    type_id: 1,
                    ..Default::default()
                },
                TypeApplication {
                    name: String::from("MoscowMule"),
                    type_id: 2,
                    ..Default::default()
                },
            ]),
            ..Default::default()
        };
        let types = [
            (0, p.clone()),
            (
                1,
                TypeDeclaration {
                    type_id: 1,
                    type_field: String::from("u64"),
                    ..Default::default()
                },
            ),
            (
                2,
                TypeDeclaration {
                    type_id: 2,
                    type_field: String::from("bool"),
                    ..Default::default()
                },
            ),
        ]
        .into_iter()
        .collect::<HashMap<_, _>>();
        let actual = expand_custom_enum(
            &FullTypeDeclaration::from_counterpart(&p, &types),
            &HashSet::default(),
        )?
        .code;

        let expected = quote! {
            #[allow(clippy::enum_variant_names)]
            #[derive(Clone, Debug, Eq, PartialEq)]
            pub enum MatchaTea<> {
                LongIsland(u64),
                MoscowMule(bool)
            }
            impl<> ::fuels::core::traits::Parameterize for self::MatchaTea<> {
                fn param_type() -> ::fuels::types::param_types::ParamType {
                    let variants = [
                        (
                            "LongIsland".to_string(),
                            <u64 as ::fuels::core::traits::Parameterize>::param_type()
                        ),
                        (
                            "MoscowMule".to_string(),
                            <bool as ::fuels::core::traits::Parameterize>::param_type()
                        )
                    ]
                    .to_vec();
                    let variants = ::fuels::types::enum_variants::EnumVariants::new(variants)
                        .unwrap_or_else(|_| panic!("{} has no variants which isn't allowed!", "MatchaTea"));
                    ::fuels::types::param_types::ParamType::Enum {
                        name: "MatchaTea".to_string(),
                        variants,
                        generics: [].to_vec()
                    }
                }
            }
            impl<> ::fuels::core::traits::Tokenizable for self::MatchaTea<> {
                fn from_token(
                    token: ::fuels::types::Token
                ) -> ::std::result::Result<Self, ::fuels::types::errors::Error>
                where
                    Self: Sized,
                {
                    let gen_err = |msg| {
                        ::fuels::types::errors::Error::InvalidData(format!(
                            "Error while instantiating {} from token! {}",
                            "MatchaTea", msg
                        ))
                    };
                    match token {
                        ::fuels::types::Token::Enum(selector) => {
                            let (discriminant, variant_token, _) = *selector;
                            match discriminant {
                                0u8 => ::std::result::Result::Ok(Self::LongIsland(
                                    ::fuels::core::traits::Tokenizable::from_token(variant_token)?
                                )),
                                1u8 => ::std::result::Result::Ok(Self::MoscowMule(
                                    ::fuels::core::traits::Tokenizable::from_token(variant_token)?
                                )),
                                _ => ::std::result::Result::Err(gen_err(format!(
                                    "Discriminant {} doesn't point to any of the enums variants.",
                                    discriminant
                                ))),
                            }
                        }
                        _ => ::std::result::Result::Err(gen_err(format!(
                            "Given token ({}) is not of the type Token::Enum!",
                            token
                        ))),
                    }
                }
                fn into_token(self) -> ::fuels::types::Token {
                    let (discriminant, token) = match self {
                        Self::LongIsland(inner) => (0u8, ::fuels::core::traits::Tokenizable::into_token(inner)),
                        Self::MoscowMule(inner) => (1u8, ::fuels::core::traits::Tokenizable::into_token(inner))
                    };
                    let variants = match < Self as :: fuels :: core :: traits :: Parameterize > :: param_type () { :: fuels :: types :: param_types :: ParamType :: Enum { variants , .. } => variants , other => panic ! ("Calling {}::param_type() must return a ParamType::Enum but instead it returned: {:?}" , "MatchaTea" , other) } ;
                    ::fuels::types::Token::Enum(::std::boxed::Box::new((discriminant, token, variants)))
                }
            }
            impl<> TryFrom<&[u8]> for self::MatchaTea<> {
                type Error = ::fuels::types::errors::Error;
                fn try_from(bytes: &[u8]) -> ::std::result::Result<Self, Self::Error> {
                    ::fuels::core::try_from_bytes(bytes)
                }
            }
            impl<> TryFrom<&::std::vec::Vec<u8>> for self::MatchaTea<> {
                type Error = ::fuels::types::errors::Error;
                fn try_from(bytes: &::std::vec::Vec<u8>) -> ::std::result::Result<Self, Self::Error> {
                    ::fuels::core::try_from_bytes(&bytes)
                }
            }
            impl<> TryFrom<::std::vec::Vec<u8>> for self::MatchaTea<> {
                type Error = ::fuels::types::errors::Error;
                fn try_from(bytes: ::std::vec::Vec<u8>) -> ::std::result::Result<Self, Self::Error> {
                    ::fuels::core::try_from_bytes(&bytes)
                }
            }
        };

        assert_eq!(actual.to_string(), expected.to_string());
        Ok(())
    }

    #[test]
    fn test_enum_with_no_variants_cannot_be_constructed() -> anyhow::Result<()> {
        let p = TypeDeclaration {
            type_id: 0,
            type_field: "enum SomeEmptyEnum".to_string(),
            components: Some(vec![]),
            ..Default::default()
        };
        let types = [(0, p.clone())].into_iter().collect::<HashMap<_, _>>();

        let err = expand_custom_enum(
            &FullTypeDeclaration::from_counterpart(&p, &types),
            &HashSet::default(),
        )
        .err()
        .ok_or_else(|| anyhow!("Was able to construct an enum without variants"))?;

        assert!(
            matches!(err, Error::InvalidData(_)),
            "Expected the error to be of the type 'InvalidData'"
        );

        Ok(())
    }

    #[test]
    fn test_expand_struct_inside_enum() -> Result<(), Error> {
        let inner_struct = TypeApplication {
            name: String::from("Infrastructure"),
            type_id: 1,
            ..Default::default()
        };
        let enum_components = vec![
            inner_struct,
            TypeApplication {
                name: "Service".to_string(),
                type_id: 2,
                ..Default::default()
            },
        ];
        let p = TypeDeclaration {
            type_id: 0,
            type_field: String::from("enum Amsterdam"),
            components: Some(enum_components),
            ..Default::default()
        };

        let types = [
            (0, p.clone()),
            (
                1,
                TypeDeclaration {
                    type_id: 1,
                    type_field: String::from("struct Building"),
                    components: Some(vec![
                        TypeApplication {
                            name: String::from("Rooms"),
                            type_id: 3,
                            ..Default::default()
                        },
                        TypeApplication {
                            name: String::from("Floors"),
                            type_id: 4,
                            ..Default::default()
                        },
                    ]),
                    ..Default::default()
                },
            ),
            (
                2,
                TypeDeclaration {
                    type_id: 2,
                    type_field: String::from("u32"),
                    ..Default::default()
                },
            ),
            (
                3,
                TypeDeclaration {
                    type_id: 3,
                    type_field: String::from("u8"),
                    ..Default::default()
                },
            ),
            (
                4,
                TypeDeclaration {
                    type_id: 4,
                    type_field: String::from("u16"),
                    ..Default::default()
                },
            ),
        ]
        .into_iter()
        .collect::<HashMap<_, _>>();
        let actual = expand_custom_enum(
            &FullTypeDeclaration::from_counterpart(&p, &types),
            &HashSet::default(),
        )?
        .code;

        let expected = quote! {
            #[allow(clippy::enum_variant_names)]
            #[derive(Clone, Debug, Eq, PartialEq)]
            pub enum Amsterdam<> {
                Infrastructure(self::Building),
                Service(u32)
            }
            impl<> ::fuels::core::traits::Parameterize for self::Amsterdam<> {
                fn param_type() -> ::fuels::types::param_types::ParamType {
                    let variants = [
                        (
                            "Infrastructure".to_string(),
                            <self::Building as ::fuels::core::traits::Parameterize>::param_type()
                        ),
                        (
                            "Service".to_string(),
                            <u32 as ::fuels::core::traits::Parameterize>::param_type()
                        )
                    ]
                    .to_vec();
                    let variants = ::fuels::types::enum_variants::EnumVariants::new(variants)
                        .unwrap_or_else(|_| panic!("{} has no variants which isn't allowed!", "Amsterdam"));
                    ::fuels::types::param_types::ParamType::Enum {
                        name: "Amsterdam".to_string(),
                        variants,
                        generics: [].to_vec()
                    }
                }
            }
            impl<> ::fuels::core::traits::Tokenizable for self::Amsterdam<> {
                fn from_token(
                    token: ::fuels::types::Token
                ) -> ::std::result::Result<Self, ::fuels::types::errors::Error>
                where
                    Self: Sized,
                {
                    let gen_err = |msg| {
                        ::fuels::types::errors::Error::InvalidData(format!(
                            "Error while instantiating {} from token! {}",
                            "Amsterdam", msg
                        ))
                    };
                    match token {
                        ::fuels::types::Token::Enum(selector) => {
                            let (discriminant, variant_token, _) = *selector;
                            match discriminant {
                                0u8 => ::std::result::Result::Ok(Self::Infrastructure(
                                    ::fuels::core::traits::Tokenizable::from_token(variant_token)?
                                )),
                                1u8 => ::std::result::Result::Ok(Self::Service(
                                    ::fuels::core::traits::Tokenizable::from_token(variant_token)?
                                )),
                                _ => ::std::result::Result::Err(gen_err(format!(
                                    "Discriminant {} doesn't point to any of the enums variants.",
                                    discriminant
                                ))),
                            }
                        }
                        _ => ::std::result::Result::Err(gen_err(format!(
                            "Given token ({}) is not of the type Token::Enum!",
                            token
                        ))),
                    }
                }
                fn into_token(self) -> ::fuels::types::Token {
                    let (discriminant, token) = match self {
                        Self::Infrastructure(inner) => (0u8, ::fuels::core::traits::Tokenizable::into_token(inner)),
                        Self::Service(inner) => (1u8, ::fuels::core::traits::Tokenizable::into_token(inner))
                    };
                    let variants = match < Self as :: fuels :: core :: traits :: Parameterize > :: param_type () { :: fuels :: types :: param_types :: ParamType :: Enum { variants , .. } => variants , other => panic ! ("Calling {}::param_type() must return a ParamType::Enum but instead it returned: {:?}" , "Amsterdam" , other) } ;
                    ::fuels::types::Token::Enum(::std::boxed::Box::new((discriminant, token, variants)))
                }
            }
            impl<> TryFrom<&[u8]> for self::Amsterdam<> {
                type Error = ::fuels::types::errors::Error;
                fn try_from(bytes: &[u8]) -> ::std::result::Result<Self, Self::Error> {
                    ::fuels::core::try_from_bytes(bytes)
                }
            }
            impl<> TryFrom<&::std::vec::Vec<u8>> for self::Amsterdam<> {
                type Error = ::fuels::types::errors::Error;
                fn try_from(bytes: &::std::vec::Vec<u8>) -> ::std::result::Result<Self, Self::Error> {
                    ::fuels::core::try_from_bytes(&bytes)
                }
            }
            impl<> TryFrom<::std::vec::Vec<u8>> for self::Amsterdam<> {
                type Error = ::fuels::types::errors::Error;
                fn try_from(bytes: ::std::vec::Vec<u8>) -> ::std::result::Result<Self, Self::Error> {
                    ::fuels::core::try_from_bytes(&bytes)
                }
            }
        };

        assert_eq!(actual.to_string(), expected.to_string());
        Ok(())
    }

    #[test]
    fn test_expand_array_inside_enum() -> Result<(), Error> {
        let enum_components = vec![TypeApplication {
            name: "SomeArr".to_string(),
            type_id: 1,
            ..Default::default()
        }];
        let p = TypeDeclaration {
            type_id: 0,
            type_field: String::from("enum SomeEnum"),
            components: Some(enum_components),
            ..Default::default()
        };
        let types = [
            (0, p.clone()),
            (
                1,
                TypeDeclaration {
                    type_id: 1,
                    type_field: "[u64; 7]".to_string(),
                    components: Some(vec![TypeApplication {
                        type_id: 2,
                        ..Default::default()
                    }]),
                    ..Default::default()
                },
            ),
            (
                2,
                TypeDeclaration {
                    type_id: 2,
                    type_field: "u64".to_string(),
                    ..Default::default()
                },
            ),
        ]
        .into_iter()
        .collect::<HashMap<_, _>>();
        let actual = expand_custom_enum(
            &FullTypeDeclaration::from_counterpart(&p, &types),
            &HashSet::default(),
        )?
        .code;

        let expected = quote! {
            #[allow(clippy::enum_variant_names)]
            #[derive(Clone, Debug, Eq, PartialEq)]
            pub enum SomeEnum < > {
                SomeArr([u64; 7usize])
            }
            impl < > ::fuels::core::traits::Parameterize for self::SomeEnum < > {
                fn param_type() -> ::fuels::types::param_types::ParamType {
                    let variants = [(
                        "SomeArr".to_string(),
                        <[u64; 7usize] as ::fuels::core::traits::Parameterize>::param_type()
                    )]
                    .to_vec();
                    let variants = ::fuels::types::enum_variants::EnumVariants::new(variants)
                        .unwrap_or_else(|_| panic!("{} has no variants which isn't allowed!", "SomeEnum"));
                    ::fuels::types::param_types::ParamType::Enum {
                        name: "SomeEnum".to_string(),
                        variants,
                        generics: [].to_vec()
                    }
                }
            }
            impl < > ::fuels::core::traits::Tokenizable for self::SomeEnum < > {
                fn from_token(
                    token: ::fuels::types::Token
                ) -> ::std::result::Result<Self, ::fuels::types::errors::Error>
                where
                    Self: Sized,
                {
                    let gen_err = |msg| {
                        ::fuels::types::errors::Error::InvalidData(format!(
                            "Error while instantiating {} from token! {}",
                            "SomeEnum", msg
                        ))
                    };
                    match token {
                        ::fuels::types::Token::Enum(selector) => {
                            let (discriminant, variant_token, _) = *selector;
                            match discriminant {
                                0u8 => ::std::result::Result::Ok(Self::SomeArr(
                                    ::fuels::core::traits::Tokenizable::from_token(variant_token)?
                                )),
                                _ => ::std::result::Result::Err(gen_err(format!(
                                    "Discriminant {} doesn't point to any of the enums variants.",
                                    discriminant
                                ))),
                            }
                        }
                        _ => ::std::result::Result::Err(gen_err(format!(
                            "Given token ({}) is not of the type Token::Enum!",
                            token
                        ))),
                    }
                }
                fn into_token(self) -> ::fuels::types::Token {
                    let (discriminant, token) = match self {
                        Self::SomeArr(inner) => (0u8, ::fuels::core::traits::Tokenizable::into_token(inner))
                    };
                    let variants = match < Self as :: fuels :: core :: traits :: Parameterize > :: param_type () { :: fuels :: types :: param_types :: ParamType :: Enum { variants , .. } => variants , other => panic ! ("Calling {}::param_type() must return a ParamType::Enum but instead it returned: {:?}" , "SomeEnum" , other) } ;
                    ::fuels::types::Token::Enum(::std::boxed::Box::new((discriminant, token, variants)))
                }
            }
            impl <> TryFrom<&[u8]> for self::SomeEnum < > {
                type Error = ::fuels::types::errors::Error;
                fn try_from(bytes: &[u8]) -> ::std::result::Result<Self, Self::Error> {
                    ::fuels::core::try_from_bytes(bytes)
                }
            }
            impl <> TryFrom<&::std::vec::Vec<u8>> for self::SomeEnum <>{
                type Error = ::fuels::types::errors::Error;
                fn try_from(bytes: &::std::vec::Vec<u8>) -> ::std::result::Result<Self, Self::Error> {
                    ::fuels::core::try_from_bytes(&bytes)
                }
            }
            impl <> TryFrom<::std::vec::Vec<u8>> for self::SomeEnum <>{
                type Error = ::fuels::types::errors::Error;
                fn try_from(bytes: ::std::vec::Vec<u8>) -> ::std::result::Result<Self, Self::Error> {
                    ::fuels::core::try_from_bytes(&bytes)
                }
            }
        };

        assert_eq!(actual.to_string(), expected.to_string());
        Ok(())
    }

    #[test]
    fn test_expand_custom_enum_with_enum() -> Result<(), Error> {
        let p = TypeDeclaration {
            type_id: 3,
            type_field: String::from("enum EnumLevel3"),
            components: Some(vec![TypeApplication {
                name: String::from("El2"),
                type_id: 2,
                ..Default::default()
            }]),
            ..Default::default()
        };
        let types = [
            (3, p.clone()),
            (
                2,
                TypeDeclaration {
                    type_id: 2,
                    type_field: String::from("enum EnumLevel2"),
                    components: Some(vec![TypeApplication {
                        name: String::from("El1"),
                        type_id: 1,
                        ..Default::default()
                    }]),
                    ..Default::default()
                },
            ),
            (
                1,
                TypeDeclaration {
                    type_id: 1,
                    type_field: String::from("enum EnumLevel1"),
                    components: Some(vec![TypeApplication {
                        name: String::from("Num"),
                        type_id: 0,
                        ..Default::default()
                    }]),
                    ..Default::default()
                },
            ),
            (
                0,
                TypeDeclaration {
                    type_id: 0,
                    type_field: String::from("u32"),
                    ..Default::default()
                },
            ),
        ]
        .into_iter()
        .collect::<HashMap<_, _>>();
        let actual = expand_custom_enum(
            &FullTypeDeclaration::from_counterpart(&p, &types),
            &HashSet::default(),
        )?
        .code;

        let expected = quote! {
            #[allow(clippy::enum_variant_names)]
            #[derive(Clone, Debug, Eq, PartialEq)]
            pub enum EnumLevel3<> {
                El2(self::EnumLevel2)
            }
            impl<> ::fuels::core::traits::Parameterize for self::EnumLevel3<> {
                fn param_type() -> ::fuels::types::param_types::ParamType {
                    let variants = [(
                        "El2".to_string(),
                        <self::EnumLevel2 as ::fuels::core::traits::Parameterize>::param_type()
                    )]
                    .to_vec();
                    let variants = ::fuels::types::enum_variants::EnumVariants::new(variants)
                        .unwrap_or_else(|_| panic!("{} has no variants which isn't allowed!", "EnumLevel3"));
                    ::fuels::types::param_types::ParamType::Enum {
                        name: "EnumLevel3".to_string(),
                        variants,
                        generics: [].to_vec()
                    }
                }
            }
            impl<> ::fuels::core::traits::Tokenizable for self::EnumLevel3<> {
                fn from_token(
                    token: ::fuels::types::Token
                ) -> ::std::result::Result<Self, ::fuels::types::errors::Error>
                where
                    Self: Sized,
                {
                    let gen_err = |msg| {
                        ::fuels::types::errors::Error::InvalidData(format!(
                            "Error while instantiating {} from token! {}",
                            "EnumLevel3", msg
                        ))
                    };
                    match token {
                        ::fuels::types::Token::Enum(selector) => {
                            let (discriminant, variant_token, _) = *selector;
                            match discriminant {
                                0u8 => ::std::result::Result::Ok(Self::El2(
                                    ::fuels::core::traits::Tokenizable::from_token(variant_token)?
                                )),
                                _ => ::std::result::Result::Err(gen_err(format!(
                                    "Discriminant {} doesn't point to any of the enums variants.",
                                    discriminant
                                ))),
                            }
                        }
                        _ => ::std::result::Result::Err(gen_err(format!(
                            "Given token ({}) is not of the type Token::Enum!",
                            token
                        ))),
                    }
                }
                fn into_token(self) -> ::fuels::types::Token {
                    let (discriminant, token) = match self {
                        Self::El2(inner) => (0u8, ::fuels::core::traits::Tokenizable::into_token(inner))
                    };
                    let variants = match < Self as :: fuels :: core :: traits :: Parameterize > :: param_type () { :: fuels :: types :: param_types :: ParamType :: Enum { variants , .. } => variants , other => panic ! ("Calling {}::param_type() must return a ParamType::Enum but instead it returned: {:?}" , "EnumLevel3" , other) } ;
                    ::fuels::types::Token::Enum(::std::boxed::Box::new((discriminant, token, variants)))
                }
            }
            impl<> TryFrom<&[u8]> for self::EnumLevel3<> {
                type Error = ::fuels::types::errors::Error;
                fn try_from(bytes: &[u8]) -> ::std::result::Result<Self, Self::Error> {
                    ::fuels::core::try_from_bytes(bytes)
                }
            }
            impl<> TryFrom<&::std::vec::Vec<u8>> for self::EnumLevel3<> {
                type Error = ::fuels::types::errors::Error;
                fn try_from(bytes: &::std::vec::Vec<u8>) -> ::std::result::Result<Self, Self::Error> {
                    ::fuels::core::try_from_bytes(&bytes)
                }
            }
            impl<> TryFrom<::std::vec::Vec<u8>> for self::EnumLevel3<> {
                type Error = ::fuels::types::errors::Error;
                fn try_from(bytes: ::std::vec::Vec<u8>) -> ::std::result::Result<Self, Self::Error> {
                    ::fuels::core::try_from_bytes(&bytes)
                }
            }
        };

        assert_eq!(actual.to_string(), expected.to_string());
        Ok(())
    }

    #[test]
    fn test_expand_custom_struct() -> Result<(), Error> {
        let p = TypeDeclaration {
            type_field: String::from("struct Cocktail"),
            components: Some(vec![
                TypeApplication {
                    name: String::from("long_island"),
                    type_id: 1,
                    ..Default::default()
                },
                TypeApplication {
                    name: String::from("cosmopolitan"),
                    type_id: 2,
                    ..Default::default()
                },
                TypeApplication {
                    name: String::from("mojito"),
                    type_id: 3,
                    ..Default::default()
                },
            ]),
            ..Default::default()
        };
        let types = [
            (0, p.clone()),
            (
                1,
                TypeDeclaration {
                    type_id: 1,
                    type_field: String::from("bool"),
                    ..Default::default()
                },
            ),
            (
                2,
                TypeDeclaration {
                    type_id: 2,
                    type_field: String::from("u64"),
                    ..Default::default()
                },
            ),
            (
                3,
                TypeDeclaration {
                    type_id: 3,
                    type_field: String::from("u32"),
                    ..Default::default()
                },
            ),
        ]
        .into_iter()
        .collect::<HashMap<_, _>>();
        let actual = expand_custom_struct(
            &FullTypeDeclaration::from_counterpart(&p, &types),
            &HashSet::default(),
        )?
        .code
        .to_string();
        let expected = TokenStream::from_str(
            r#"
            # [derive (Clone , Debug , Eq , PartialEq)] pub struct Cocktail < > { pub long_island : bool , pub cosmopolitan : u64 , pub mojito : u32 } impl < > :: fuels :: core :: traits :: Parameterize for self :: Cocktail < > { fn param_type () -> :: fuels :: types :: param_types :: ParamType { let types = [("long_island" . to_string () , < bool as :: fuels :: core :: traits :: Parameterize > :: param_type ()) , ("cosmopolitan" . to_string () , < u64 as :: fuels :: core :: traits :: Parameterize > :: param_type ()) , ("mojito" . to_string () , < u32 as :: fuels :: core :: traits :: Parameterize > :: param_type ())] . to_vec () ; :: fuels :: types :: param_types :: ParamType :: Struct { name : "Cocktail" . to_string () , fields : types , generics : [] . to_vec () } } } impl < > :: fuels :: core :: traits :: Tokenizable for self :: Cocktail < > { fn into_token (self) -> :: fuels :: types :: Token { let tokens = [self . long_island . into_token () , self . cosmopolitan . into_token () , self . mojito . into_token ()] . to_vec () ; :: fuels :: types :: Token :: Struct (tokens) } fn from_token (token : :: fuels :: types :: Token) -> :: std :: result :: Result < Self , :: fuels :: types :: errors :: Error > { match token { :: fuels :: types :: Token :: Struct (tokens) => { let mut tokens_iter = tokens . into_iter () ; let mut next_token = move || { tokens_iter . next () . ok_or_else (|| { :: fuels :: types :: errors :: Error :: InstantiationError (format ! ("Ran out of tokens before '{}' has finished construction!" , "Cocktail")) }) } ; :: std :: result :: Result :: Ok (Self { long_island : :: fuels :: core :: traits :: Tokenizable :: from_token (next_token () ?) ? , cosmopolitan : :: fuels :: core :: traits :: Tokenizable :: from_token (next_token () ?) ? , mojito : :: fuels :: core :: traits :: Tokenizable :: from_token (next_token () ?) ? , }) } , other => :: std :: result :: Result :: Err (:: fuels :: types :: errors :: Error :: InstantiationError (format ! ("Error while constructing '{}'. Expected token of type Token::Struct, got {:?}" , "Cocktail" , other))) , } } } impl < > TryFrom < & [u8] > for self :: Cocktail < > { type Error = :: fuels :: types :: errors :: Error ; fn try_from (bytes : & [u8]) -> :: std :: result :: Result < Self , Self :: Error > { :: fuels :: core :: try_from_bytes (bytes) } } impl < > TryFrom < & :: std :: vec :: Vec < u8 >> for self :: Cocktail < > { type Error = :: fuels :: types :: errors :: Error ; fn try_from (bytes : & :: std :: vec :: Vec < u8 >) -> :: std :: result :: Result < Self , Self :: Error > { :: fuels :: core :: try_from_bytes (& bytes) } } impl < > TryFrom < :: std :: vec :: Vec < u8 >> for self :: Cocktail < > { type Error = :: fuels :: types :: errors :: Error ; fn try_from (bytes : :: std :: vec :: Vec < u8 >) -> :: std :: result :: Result < Self , Self :: Error > { :: fuels :: core :: try_from_bytes (& bytes) } }
            "#,
        )?.to_string();

        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn test_struct_with_no_fields_can_be_constructed() -> Result<(), Error> {
        let p = TypeDeclaration {
            type_id: 0,
            type_field: "struct SomeEmptyStruct".to_string(),
            components: Some(vec![]),
            ..Default::default()
        };
        let types = [(0, p.clone())].into_iter().collect::<HashMap<_, _>>();

        let actual = expand_custom_struct(
            &FullTypeDeclaration::from_counterpart(&p, &types),
            &HashSet::default(),
        )?
        .code
        .to_string();

        let expected = TokenStream::from_str(
            r#"
            # [derive (Clone , Debug , Eq , PartialEq)] pub struct SomeEmptyStruct < > { } impl < > :: fuels :: core :: traits :: Parameterize for self :: SomeEmptyStruct < > { fn param_type () -> :: fuels :: types :: param_types :: ParamType { let types = [] . to_vec () ; :: fuels :: types :: param_types :: ParamType :: Struct { name : "SomeEmptyStruct" . to_string () , fields : types , generics : [] . to_vec () } } } impl < > :: fuels :: core :: traits :: Tokenizable for self :: SomeEmptyStruct < > { fn into_token (self) -> :: fuels :: types :: Token { let tokens = [] . to_vec () ; :: fuels :: types :: Token :: Struct (tokens) } fn from_token (token : :: fuels :: types :: Token) -> :: std :: result :: Result < Self , :: fuels :: types :: errors :: Error > { match token { :: fuels :: types :: Token :: Struct (tokens) => { let mut tokens_iter = tokens . into_iter () ; let mut next_token = move || { tokens_iter . next () . ok_or_else (|| { :: fuels :: types :: errors :: Error :: InstantiationError (format ! ("Ran out of tokens before '{}' has finished construction!" , "SomeEmptyStruct")) }) } ; :: std :: result :: Result :: Ok (Self { }) } , other => :: std :: result :: Result :: Err (:: fuels :: types :: errors :: Error :: InstantiationError (format ! ("Error while constructing '{}'. Expected token of type Token::Struct, got {:?}" , "SomeEmptyStruct" , other))) , } } } impl < > TryFrom < & [u8] > for self :: SomeEmptyStruct < > { type Error = :: fuels :: types :: errors :: Error ; fn try_from (bytes : & [u8]) -> :: std :: result :: Result < Self , Self :: Error > { :: fuels :: core :: try_from_bytes (bytes) } } impl < > TryFrom < & :: std :: vec :: Vec < u8 >> for self :: SomeEmptyStruct < > { type Error = :: fuels :: types :: errors :: Error ; fn try_from (bytes : & :: std :: vec :: Vec < u8 >) -> :: std :: result :: Result < Self , Self :: Error > { :: fuels :: core :: try_from_bytes (& bytes) } } impl < > TryFrom < :: std :: vec :: Vec < u8 >> for self :: SomeEmptyStruct < > { type Error = :: fuels :: types :: errors :: Error ; fn try_from (bytes : :: std :: vec :: Vec < u8 >) -> :: std :: result :: Result < Self , Self :: Error > { :: fuels :: core :: try_from_bytes (& bytes) } }
            "#,
        )?.to_string();

        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn test_expand_custom_struct_with_struct() -> Result<(), Error> {
        let p = TypeDeclaration {
            type_id: 0,
            type_field: String::from("struct Cocktail"),
            components: Some(vec![
                TypeApplication {
                    name: String::from("long_island"),
                    type_id: 1,
                    ..Default::default()
                },
                TypeApplication {
                    name: String::from("mojito"),
                    type_id: 4,
                    ..Default::default()
                },
            ]),
            ..Default::default()
        };
        let types = [
            (0, p.clone()),
            (
                1,
                TypeDeclaration {
                    type_id: 1,
                    type_field: String::from("struct Shaker"),
                    components: Some(vec![
                        TypeApplication {
                            name: String::from("cosmopolitan"),
                            type_id: 2,
                            ..Default::default()
                        },
                        TypeApplication {
                            name: String::from("bimbap"),
                            type_id: 3,
                            ..Default::default()
                        },
                    ]),
                    ..Default::default()
                },
            ),
            (
                2,
                TypeDeclaration {
                    type_id: 2,
                    type_field: String::from("bool"),
                    ..Default::default()
                },
            ),
            (
                3,
                TypeDeclaration {
                    type_id: 3,
                    type_field: String::from("u64"),
                    ..Default::default()
                },
            ),
            (
                4,
                TypeDeclaration {
                    type_id: 4,
                    type_field: String::from("u32"),
                    ..Default::default()
                },
            ),
        ]
        .into_iter()
        .collect::<HashMap<_, _>>();
        let actual = expand_custom_struct(
            &FullTypeDeclaration::from_counterpart(&p, &types),
            &HashSet::default(),
        )?
        .code
        .to_string();
        let expected = TokenStream::from_str(
            r#"
            # [derive (Clone , Debug , Eq , PartialEq)] pub struct Cocktail < > { pub long_island : self :: Shaker , pub mojito : u32 } impl < > :: fuels :: core :: traits :: Parameterize for self :: Cocktail < > { fn param_type () -> :: fuels :: types :: param_types :: ParamType { let types = [("long_island" . to_string () , < self :: Shaker as :: fuels :: core :: traits :: Parameterize > :: param_type ()) , ("mojito" . to_string () , < u32 as :: fuels :: core :: traits :: Parameterize > :: param_type ())] . to_vec () ; :: fuels :: types :: param_types :: ParamType :: Struct { name : "Cocktail" . to_string () , fields : types , generics : [] . to_vec () } } } impl < > :: fuels :: core :: traits :: Tokenizable for self :: Cocktail < > { fn into_token (self) -> :: fuels :: types :: Token { let tokens = [self . long_island . into_token () , self . mojito . into_token ()] . to_vec () ; :: fuels :: types :: Token :: Struct (tokens) } fn from_token (token : :: fuels :: types :: Token) -> :: std :: result :: Result < Self , :: fuels :: types :: errors :: Error > { match token { :: fuels :: types :: Token :: Struct (tokens) => { let mut tokens_iter = tokens . into_iter () ; let mut next_token = move || { tokens_iter . next () . ok_or_else (|| { :: fuels :: types :: errors :: Error :: InstantiationError (format ! ("Ran out of tokens before '{}' has finished construction!" , "Cocktail")) }) } ; :: std :: result :: Result :: Ok (Self { long_island : :: fuels :: core :: traits :: Tokenizable :: from_token (next_token () ?) ? , mojito : :: fuels :: core :: traits :: Tokenizable :: from_token (next_token () ?) ? , }) } , other => :: std :: result :: Result :: Err (:: fuels :: types :: errors :: Error :: InstantiationError (format ! ("Error while constructing '{}'. Expected token of type Token::Struct, got {:?}" , "Cocktail" , other))) , } } } impl < > TryFrom < & [u8] > for self :: Cocktail < > { type Error = :: fuels :: types :: errors :: Error ; fn try_from (bytes : & [u8]) -> :: std :: result :: Result < Self , Self :: Error > { :: fuels :: core :: try_from_bytes (bytes) } } impl < > TryFrom < & :: std :: vec :: Vec < u8 >> for self :: Cocktail < > { type Error = :: fuels :: types :: errors :: Error ; fn try_from (bytes : & :: std :: vec :: Vec < u8 >) -> :: std :: result :: Result < Self , Self :: Error > { :: fuels :: core :: try_from_bytes (& bytes) } } impl < > TryFrom < :: std :: vec :: Vec < u8 >> for self :: Cocktail < > { type Error = :: fuels :: types :: errors :: Error ; fn try_from (bytes : :: std :: vec :: Vec < u8 >) -> :: std :: result :: Result < Self , Self :: Error > { :: fuels :: core :: try_from_bytes (& bytes) } }
            "#,
        )?.to_string();

        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn test_expand_struct_new_abi() -> Result<(), Error> {
        let s = r#"
            {
                "types": [
                  {
                    "typeId": 6,
                    "type": "u64",
                    "components": null,
                    "typeParameters": null
                  },
                  {
                    "typeId": 8,
                    "type": "b256",
                    "components": null,
                    "typeParameters": null
                  },
                  {
                    "typeId": 6,
                    "type": "u64",
                    "components": null,
                    "typeParameters": null
                  },
                  {
                    "typeId": 8,
                    "type": "b256",
                    "components": null,
                    "typeParameters": null
                  },
                  {
                    "typeId": 10,
                    "type": "bool",
                    "components": null,
                    "typeParameters": null
                  },
                  {
                    "typeId": 12,
                    "type": "struct MyStruct1",
                    "components": [
                      {
                        "name": "x",
                        "type": 6,
                        "typeArguments": null
                      },
                      {
                        "name": "y",
                        "type": 8,
                        "typeArguments": null
                      }
                    ],
                    "typeParameters": null
                  },
                  {
                    "typeId": 6,
                    "type": "u64",
                    "components": null,
                    "typeParameters": null
                  },
                  {
                    "typeId": 8,
                    "type": "b256",
                    "components": null,
                    "typeParameters": null
                  },
                  {
                    "typeId": 2,
                    "type": "struct MyStruct1",
                    "components": [
                      {
                        "name": "x",
                        "type": 6,
                        "typeArguments": null
                      },
                      {
                        "name": "y",
                        "type": 8,
                        "typeArguments": null
                      }
                    ],
                    "typeParameters": null
                  },
                  {
                    "typeId": 3,
                    "type": "struct MyStruct2",
                    "components": [
                      {
                        "name": "x",
                        "type": 10,
                        "typeArguments": null
                      },
                      {
                        "name": "y",
                        "type": 12,
                        "typeArguments": []
                      }
                    ],
                    "typeParameters": null
                  },
                  {
                    "typeId": 26,
                    "type": "struct MyStruct1",
                    "components": [
                      {
                        "name": "x",
                        "type": 6,
                        "typeArguments": null
                      },
                      {
                        "name": "y",
                        "type": 8,
                        "typeArguments": null
                      }
                    ],
                    "typeParameters": null
                  }
                ],
                "functions": [
                  {
                    "type": "function",
                    "inputs": [
                      {
                        "name": "s1",
                        "type": 2,
                        "typeArguments": []
                      },
                      {
                        "name": "s2",
                        "type": 3,
                        "typeArguments": []
                      }
                    ],
                    "name": "some_abi_funct",
                    "output": {
                      "name": "",
                      "type": 26,
                      "typeArguments": []
                    }
                  }
                ]
              }
    "#;
        let parsed_abi: ProgramABI = serde_json::from_str(s)?;
        let types = parsed_abi
            .types
            .into_iter()
            .map(|t| (t.type_id, t))
            .collect::<HashMap<usize, TypeDeclaration>>();

        let s1 = types.get(&2).unwrap();

        let actual = expand_custom_struct(
            &FullTypeDeclaration::from_counterpart(s1, &types),
            &HashSet::default(),
        )?
        .code
        .to_string();

        let expected = TokenStream::from_str(
            r#"
            # [derive (Clone , Debug , Eq , PartialEq)] pub struct MyStruct1 < > { pub x : u64 , pub y : :: fuels :: types :: Bits256 } impl < > :: fuels :: core :: traits :: Parameterize for self :: MyStruct1 < > { fn param_type () -> :: fuels :: types :: param_types :: ParamType { let types = [("x" . to_string () , < u64 as :: fuels :: core :: traits :: Parameterize > :: param_type ()) , ("y" . to_string () , < :: fuels :: types :: Bits256 as :: fuels :: core :: traits :: Parameterize > :: param_type ())] . to_vec () ; :: fuels :: types :: param_types :: ParamType :: Struct { name : "MyStruct1" . to_string () , fields : types , generics : [] . to_vec () } } } impl < > :: fuels :: core :: traits :: Tokenizable for self :: MyStruct1 < > { fn into_token (self) -> :: fuels :: types :: Token { let tokens = [self . x . into_token () , self . y . into_token ()] . to_vec () ; :: fuels :: types :: Token :: Struct (tokens) } fn from_token (token : :: fuels :: types :: Token) -> :: std :: result :: Result < Self , :: fuels :: types :: errors :: Error > { match token { :: fuels :: types :: Token :: Struct (tokens) => { let mut tokens_iter = tokens . into_iter () ; let mut next_token = move || { tokens_iter . next () . ok_or_else (|| { :: fuels :: types :: errors :: Error :: InstantiationError (format ! ("Ran out of tokens before '{}' has finished construction!" , "MyStruct1")) }) } ; :: std :: result :: Result :: Ok (Self { x : :: fuels :: core :: traits :: Tokenizable :: from_token (next_token () ?) ? , y : :: fuels :: core :: traits :: Tokenizable :: from_token (next_token () ?) ? , }) } , other => :: std :: result :: Result :: Err (:: fuels :: types :: errors :: Error :: InstantiationError (format ! ("Error while constructing '{}'. Expected token of type Token::Struct, got {:?}" , "MyStruct1" , other))) , } } } impl < > TryFrom < & [u8] > for self :: MyStruct1 < > { type Error = :: fuels :: types :: errors :: Error ; fn try_from (bytes : & [u8]) -> :: std :: result :: Result < Self , Self :: Error > { :: fuels :: core :: try_from_bytes (bytes) } } impl < > TryFrom < & :: std :: vec :: Vec < u8 >> for self :: MyStruct1 < > { type Error = :: fuels :: types :: errors :: Error ; fn try_from (bytes : & :: std :: vec :: Vec < u8 >) -> :: std :: result :: Result < Self , Self :: Error > { :: fuels :: core :: try_from_bytes (& bytes) } } impl < > TryFrom < :: std :: vec :: Vec < u8 >> for self :: MyStruct1 < > { type Error = :: fuels :: types :: errors :: Error ; fn try_from (bytes : :: std :: vec :: Vec < u8 >) -> :: std :: result :: Result < Self , Self :: Error > { :: fuels :: core :: try_from_bytes (& bytes) } }
            "#,
            )?.to_string();

        assert_eq!(actual, expected);

        let s2 = types.get(&3).unwrap();

        let actual = expand_custom_struct(
            &FullTypeDeclaration::from_counterpart(s2, &types),
            &HashSet::default(),
        )?
        .code
        .to_string();

        let expected = TokenStream::from_str(
            r#"
            # [derive (Clone , Debug , Eq , PartialEq)] pub struct MyStruct2 < > { pub x : bool , pub y : self :: MyStruct1 } impl < > :: fuels :: core :: traits :: Parameterize for self :: MyStruct2 < > { fn param_type () -> :: fuels :: types :: param_types :: ParamType { let types = [("x" . to_string () , < bool as :: fuels :: core :: traits :: Parameterize > :: param_type ()) , ("y" . to_string () , < self :: MyStruct1 as :: fuels :: core :: traits :: Parameterize > :: param_type ())] . to_vec () ; :: fuels :: types :: param_types :: ParamType :: Struct { name : "MyStruct2" . to_string () , fields : types , generics : [] . to_vec () } } } impl < > :: fuels :: core :: traits :: Tokenizable for self :: MyStruct2 < > { fn into_token (self) -> :: fuels :: types :: Token { let tokens = [self . x . into_token () , self . y . into_token ()] . to_vec () ; :: fuels :: types :: Token :: Struct (tokens) } fn from_token (token : :: fuels :: types :: Token) -> :: std :: result :: Result < Self , :: fuels :: types :: errors :: Error > { match token { :: fuels :: types :: Token :: Struct (tokens) => { let mut tokens_iter = tokens . into_iter () ; let mut next_token = move || { tokens_iter . next () . ok_or_else (|| { :: fuels :: types :: errors :: Error :: InstantiationError (format ! ("Ran out of tokens before '{}' has finished construction!" , "MyStruct2")) }) } ; :: std :: result :: Result :: Ok (Self { x : :: fuels :: core :: traits :: Tokenizable :: from_token (next_token () ?) ? , y : :: fuels :: core :: traits :: Tokenizable :: from_token (next_token () ?) ? , }) } , other => :: std :: result :: Result :: Err (:: fuels :: types :: errors :: Error :: InstantiationError (format ! ("Error while constructing '{}'. Expected token of type Token::Struct, got {:?}" , "MyStruct2" , other))) , } } } impl < > TryFrom < & [u8] > for self :: MyStruct2 < > { type Error = :: fuels :: types :: errors :: Error ; fn try_from (bytes : & [u8]) -> :: std :: result :: Result < Self , Self :: Error > { :: fuels :: core :: try_from_bytes (bytes) } } impl < > TryFrom < & :: std :: vec :: Vec < u8 >> for self :: MyStruct2 < > { type Error = :: fuels :: types :: errors :: Error ; fn try_from (bytes : & :: std :: vec :: Vec < u8 >) -> :: std :: result :: Result < Self , Self :: Error > { :: fuels :: core :: try_from_bytes (& bytes) } } impl < > TryFrom < :: std :: vec :: Vec < u8 >> for self :: MyStruct2 < > { type Error = :: fuels :: types :: errors :: Error ; fn try_from (bytes : :: std :: vec :: Vec < u8 >) -> :: std :: result :: Result < Self , Self :: Error > { :: fuels :: core :: try_from_bytes (& bytes) } }
            "#,
            )?.to_string();

        assert_eq!(actual, expected);

        Ok(())
    }

    #[test]
    fn will_skip_shared_types() {
        // given
        let types = ["struct SomeStruct", "enum SomeEnum"].map(given_a_custom_type);
        let shared_types = HashSet::from_iter(types.iter().take(1).cloned());

        // when
        let generated_code = generate_types(types, &shared_types).expect("Should have succeeded.");

        // then
        assert_eq!(
            generated_code.usable_types,
            HashSet::from([TypePath::new("SomeEnum").expect("Hand crafted, should not fail")])
        );
    }

    fn given_a_custom_type(type_field: &str) -> FullTypeDeclaration {
        FullTypeDeclaration {
            type_field: type_field.to_string(),
            components: vec![FullTypeApplication {
                name: "a".to_string(),
                type_decl: FullTypeDeclaration {
                    type_field: "u8".to_string(),
                    components: vec![],
                    type_parameters: vec![],
                },
                type_arguments: vec![],
            }],
            type_parameters: vec![],
        }
    }
}
