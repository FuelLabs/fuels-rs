use std::collections::HashSet;

use fuel_abi_types::abi::full_program::FullTypeDeclaration;
use itertools::Itertools;
use quote::quote;

use crate::{
    error::Result,
    program_bindings::{
        custom_types::{enums::expand_custom_enum, structs::expand_custom_struct},
        generated_code::GeneratedCode,
        utils::sdk_provided_custom_types_lookup,
    },
    utils::TypePath,
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
pub(crate) fn generate_types<'a, T: IntoIterator<Item = &'a FullTypeDeclaration>>(
    types: T,
    shared_types: &HashSet<FullTypeDeclaration>,
    no_std: bool,
) -> Result<GeneratedCode> {
    types
        .into_iter()
        .filter(|ttype| !should_skip_codegen(ttype))
        .map(|ttype: &FullTypeDeclaration| {
            if shared_types.contains(ttype) {
                reexport_the_shared_type(ttype, no_std)
            } else if ttype.is_struct_type() {
                expand_custom_struct(ttype, no_std)
            } else {
                expand_custom_enum(ttype, no_std)
            }
        })
        .fold_ok(GeneratedCode::default(), |acc, generated_code| {
            acc.merge(generated_code)
        })
}

/// Instead of generating bindings for `ttype` this fn will just generate a `pub use` pointing to
/// the already generated equivalent shared type.
fn reexport_the_shared_type(ttype: &FullTypeDeclaration, no_std: bool) -> Result<GeneratedCode> {
    // e.g. some_libary::another_mod::SomeStruct
    let type_path = ttype
        .custom_type_path()
        .expect("This must be a custom type due to the previous filter step");

    let type_mod = type_path.parent();

    let from_top_lvl_to_shared_types =
        TypePath::new("super::shared_types").expect("This is known to be a valid TypePath");

    let top_lvl_mod = TypePath::default();
    let from_current_mod_to_top_level = top_lvl_mod.relative_path_from(&type_mod);

    let path = from_current_mod_to_top_level
        .append(from_top_lvl_to_shared_types)
        .append(type_path);

    // e.g. pub use super::super::super::shared_types::some_library::another_mod::SomeStruct;
    let the_reexport = quote! {pub use #path;};

    Ok(GeneratedCode::new(the_reexport, Default::default(), no_std).wrap_in_mod(type_mod))
}

// Checks whether the given type should not have code generated for it. This
// is mainly because the corresponding type in Rust already exists --
// e.g. the contract's Vec type is mapped to std::vec::Vec from the Rust
// stdlib, ContractId is a custom type implemented by fuels-rs, etc.
// Others like 'std::vec::RawVec' are skipped because they are
// implementation details of the contract's Vec type and are not directly
// used in the SDK.
fn should_skip_codegen(type_decl: &FullTypeDeclaration) -> bool {
    if !type_decl.is_custom_type() {
        return true;
    }

    let type_path = type_decl.custom_type_path().unwrap();

    is_type_sdk_provided(&type_path) || is_type_unused(&type_path)
}

fn is_type_sdk_provided(type_path: &TypePath) -> bool {
    sdk_provided_custom_types_lookup().contains_key(type_path)
}

fn is_type_unused(type_path: &TypePath) -> bool {
    let msg = "Known to be correct";
    [
        // TODO: To be removed once https://github.com/FuelLabs/fuels-rs/issues/881 is unblocked.
        TypePath::new("RawBytes").expect(msg),
        TypePath::new("std::vec::RawVec").expect(msg),
        TypePath::new("std::bytes::RawBytes").expect(msg),
        // TODO: To be removed once https://github.com/FuelLabs/fuels-rs/issues/881 is unblocked.
        TypePath::new("RawVec").expect(msg),
    ]
    .contains(type_path)
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
    use std::collections::HashMap;

    use fuel_abi_types::abi::program::{ProgramABI, TypeApplication, TypeDeclaration};
    use quote::quote;

    use super::*;

    #[test]
    fn test_expand_custom_enum() -> Result<()> {
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

        let actual = expand_custom_enum(&FullTypeDeclaration::from_counterpart(&p, &types), false)?;

        let expected = quote! {
            #[allow(clippy::enum_variant_names)]
            #[derive(
                Clone,
                Debug,
                Eq,
                PartialEq,
                ::fuels::macros::Parameterize,
                ::fuels::macros::Tokenizable,
                ::fuels::macros::TryFrom
            )]
            pub enum MatchaTea<> {
                LongIsland(u64),
                MoscowMule(bool)
            }
        };

        assert_eq!(actual.code().to_string(), expected.to_string());
        Ok(())
    }

    #[test]
    fn test_enum_with_no_variants_cannot_be_constructed() -> Result<()> {
        let p = TypeDeclaration {
            type_id: 0,
            type_field: "enum SomeEmptyEnum".to_string(),
            components: Some(vec![]),
            ..Default::default()
        };
        let types = [(0, p.clone())].into_iter().collect::<HashMap<_, _>>();

        expand_custom_enum(&FullTypeDeclaration::from_counterpart(&p, &types), false)
            .expect_err("Was able to construct an enum without variants");

        Ok(())
    }

    #[test]
    fn test_expand_struct_inside_enum() -> Result<()> {
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
                    components: Some(vec![]),
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
        ]
        .into_iter()
        .collect::<HashMap<_, _>>();

        let actual = expand_custom_enum(&FullTypeDeclaration::from_counterpart(&p, &types), false)?;

        let expected = quote! {
            #[allow(clippy::enum_variant_names)]
            #[derive(
                Clone,
                Debug,
                Eq,
                PartialEq,
                ::fuels::macros::Parameterize,
                ::fuels::macros::Tokenizable,
                ::fuels::macros::TryFrom
            )]
            pub enum Amsterdam<> {
                Infrastructure(self::Building),
                Service(u32)
            }
        };

        assert_eq!(actual.code().to_string(), expected.to_string());
        Ok(())
    }

    #[test]
    fn test_expand_array_inside_enum() -> Result<()> {
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

        let actual = expand_custom_enum(&FullTypeDeclaration::from_counterpart(&p, &types), false)?;

        let expected = quote! {
            #[allow(clippy::enum_variant_names)]
            #[derive(
                Clone,
                Debug,
                Eq,
                PartialEq,
                ::fuels::macros::Parameterize,
                ::fuels::macros::Tokenizable,
                ::fuels::macros::TryFrom
            )]
            pub enum SomeEnum < > {
                SomeArr([u64; 7usize])
            }
        };

        assert_eq!(actual.code().to_string(), expected.to_string());
        Ok(())
    }

    #[test]
    fn test_expand_custom_enum_with_enum() -> Result<()> {
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

        let actual = expand_custom_enum(&FullTypeDeclaration::from_counterpart(&p, &types), false)?;

        let expected = quote! {
            #[allow(clippy::enum_variant_names)]
            #[derive(
                Clone,
                Debug,
                Eq,
                PartialEq,
                ::fuels::macros::Parameterize,
                ::fuels::macros::Tokenizable,
                ::fuels::macros::TryFrom
            )]
            pub enum EnumLevel3<> {
                El2(self::EnumLevel2)
            }
        };

        assert_eq!(actual.code().to_string(), expected.to_string());
        Ok(())
    }

    #[test]
    fn test_expand_custom_struct() -> Result<()> {
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

        let actual =
            expand_custom_struct(&FullTypeDeclaration::from_counterpart(&p, &types), false)?;

        let expected = quote! {
            #[derive(
                Clone,
                Debug,
                Eq,
                PartialEq,
                ::fuels::macros::Parameterize,
                ::fuels::macros::Tokenizable,
                ::fuels::macros::TryFrom
            )]
            pub struct Cocktail < > {
                pub long_island: bool,
                pub cosmopolitan: u64,
                pub mojito: u32
            }
        };

        assert_eq!(actual.code().to_string(), expected.to_string());

        Ok(())
    }

    #[test]
    fn test_struct_with_no_fields_can_be_constructed() -> Result<()> {
        let p = TypeDeclaration {
            type_id: 0,
            type_field: "struct SomeEmptyStruct".to_string(),
            components: Some(vec![]),
            ..Default::default()
        };
        let types = [(0, p.clone())].into_iter().collect::<HashMap<_, _>>();

        let actual =
            expand_custom_struct(&FullTypeDeclaration::from_counterpart(&p, &types), false)?;

        let expected = quote! {
            #[derive(
                Clone,
                Debug,
                Eq,
                PartialEq,
                ::fuels::macros::Parameterize,
                ::fuels::macros::Tokenizable,
                ::fuels::macros::TryFrom
            )]
            pub struct SomeEmptyStruct < > {}
        };

        assert_eq!(actual.code().to_string(), expected.to_string());

        Ok(())
    }

    #[test]
    fn test_expand_custom_struct_with_struct() -> Result<()> {
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
                    type_field: String::from("struct Shaker"),
                    components: Some(vec![]),
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
        ]
        .into_iter()
        .collect::<HashMap<_, _>>();

        let actual =
            expand_custom_struct(&FullTypeDeclaration::from_counterpart(&p, &types), false)?;

        let expected = quote! {
            #[derive(
                Clone,
                Debug,
                Eq,
                PartialEq,
                ::fuels::macros::Parameterize,
                ::fuels::macros::Tokenizable,
                ::fuels::macros::TryFrom
            )]
            pub struct Cocktail < > {
                pub long_island: self::Shaker,
                pub mojito: u32
            }
        };

        assert_eq!(actual.code().to_string(), expected.to_string());
        Ok(())
    }

    #[test]
    fn test_expand_struct_new_abi() -> Result<()> {
        let s = r#"
            {
                "types": [
                  {
                    "typeId": 0,
                    "type": "u64",
                    "components": null,
                    "typeParameters": null
                  },
                  {
                    "typeId": 1,
                    "type": "b256",
                    "components": null,
                    "typeParameters": null
                  },
                  {
                    "typeId": 2,
                    "type": "bool",
                    "components": null,
                    "typeParameters": null
                  },
                  {
                    "typeId": 3,
                    "type": "struct MyStruct1",
                    "components": [
                      {
                        "name": "x",
                        "type": 0,
                        "typeArguments": null
                      },
                      {
                        "name": "y",
                        "type": 1,
                        "typeArguments": null
                      }
                    ],
                    "typeParameters": null
                  },
                  {
                    "typeId": 4,
                    "type": "struct MyStruct2",
                    "components": [
                      {
                        "name": "x",
                        "type": 2,
                        "typeArguments": null
                      },
                      {
                        "name": "y",
                        "type": 3,
                        "typeArguments": []
                      }
                    ],
                    "typeParameters": null
                  }
                ],
                "functions": [
                  {
                    "type": "function",
                    "inputs": [],
                    "name": "some_abi_funct",
                    "output": {
                      "name": "",
                      "type": 0,
                      "typeArguments": []
                    }
                  }
                ]
            }"#;
        let parsed_abi: ProgramABI = serde_json::from_str(s)?;
        let types = parsed_abi
            .types
            .into_iter()
            .map(|t| (t.type_id, t))
            .collect::<HashMap<usize, TypeDeclaration>>();

        let s1 = types.get(&3).unwrap();

        let actual =
            expand_custom_struct(&FullTypeDeclaration::from_counterpart(s1, &types), false)?;

        let expected = quote! {
            #[derive(
                Clone,
                Debug,
                Eq,
                PartialEq,
                ::fuels::macros::Parameterize,
                ::fuels::macros::Tokenizable,
                ::fuels::macros::TryFrom
            )]
            pub struct MyStruct1 < > {
                pub x: u64,
                pub y: ::fuels::types::Bits256
            }
        };

        assert_eq!(actual.code().to_string(), expected.to_string());

        let s2 = types.get(&4).unwrap();

        let actual =
            expand_custom_struct(&FullTypeDeclaration::from_counterpart(s2, &types), false)?;

        let expected = quote! {
            #[derive(
                Clone,
                Debug,
                Eq,
                PartialEq,
                ::fuels::macros::Parameterize,
                ::fuels::macros::Tokenizable,
                ::fuels::macros::TryFrom
            )]
            pub struct MyStruct2 < > {
                pub x: bool,
                pub y: self::MyStruct1
            }
        };

        assert_eq!(actual.code().to_string(), expected.to_string());

        Ok(())
    }

    #[test]
    fn shared_types_are_just_reexported() {
        // given
        let type_decl = FullTypeDeclaration {
            type_field: "struct some_shared_lib::SharedStruct".to_string(),
            components: vec![],
            type_parameters: vec![],
        };
        let shared_types = HashSet::from([type_decl.clone()]);

        // when
        let generated_code = generate_types(&[type_decl], &shared_types, false).unwrap();

        // then
        let expected_code = quote! {
            #[allow(clippy::too_many_arguments)]
            #[no_implicit_prelude]
            pub mod some_shared_lib {
                use ::core::{
                    clone::Clone,
                    convert::{Into, TryFrom, From},
                    iter::IntoIterator,
                    iter::Iterator,
                    marker::Sized,
                    panic,
                };

                use ::std::{string::ToString, format, vec};
                pub use super::super::shared_types::some_shared_lib::SharedStruct;
            }
        };

        assert_eq!(generated_code.code().to_string(), expected_code.to_string());
    }
}
