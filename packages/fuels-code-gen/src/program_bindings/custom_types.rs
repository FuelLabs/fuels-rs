use std::collections::HashSet;

use fuel_abi_types::utils::extract_custom_type_name;
use itertools::Itertools;

use crate::{
    error::Result,
    program_bindings::{
        abi_types::FullTypeDeclaration,
        custom_types::{enums::expand_custom_enum, structs::expand_custom_struct},
        generated_code::GeneratedCode,
        utils::get_sdk_provided_types,
    },
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
    no_std: bool,
) -> Result<GeneratedCode> {
    HashSet::from_iter(types)
        .difference(shared_types)
        .filter(|ttype| !should_skip_codegen(&ttype.type_field))
        .filter_map(|ttype| {
            if ttype.is_struct_type() {
                Some(expand_custom_struct(ttype, shared_types, no_std))
            } else if ttype.is_enum_type() {
                Some(expand_custom_enum(ttype, shared_types, no_std))
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
    let name = extract_custom_type_name(type_field).unwrap_or_else(|| type_field.to_string());

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
    use std::collections::{HashMap, HashSet};

    use fuel_abi_types::program_abi::{ProgramABI, TypeApplication, TypeDeclaration};
    use quote::quote;

    use super::*;
    use crate::{program_bindings::abi_types::FullTypeApplication, utils::TypePath};

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

        let actual = expand_custom_enum(
            &FullTypeDeclaration::from_counterpart(&p, &types),
            &HashSet::default(),
            false,
        )?
        .code;

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
            #[FuelsTypesPath("::fuels::types")]
            #[FuelsCorePath("::fuels::core")]
            pub enum MatchaTea<> {
                LongIsland(u64),
                MoscowMule(bool)
            }
        };

        assert_eq!(actual.to_string(), expected.to_string());
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

        expand_custom_enum(
            &FullTypeDeclaration::from_counterpart(&p, &types),
            &HashSet::default(),
            false,
        )
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

        let actual = expand_custom_enum(
            &FullTypeDeclaration::from_counterpart(&p, &types),
            &HashSet::default(),
            false,
        )?
        .code;

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
            #[FuelsTypesPath("::fuels::types")]
            #[FuelsCorePath("::fuels::core")]
            pub enum Amsterdam<> {
                Infrastructure(self::Building),
                Service(u32)
            }
        };

        assert_eq!(actual.to_string(), expected.to_string());
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

        let actual = expand_custom_enum(
            &FullTypeDeclaration::from_counterpart(&p, &types),
            &HashSet::default(),
            false,
        )?
        .code;

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
            #[FuelsTypesPath("::fuels::types")]
            #[FuelsCorePath("::fuels::core")]
            pub enum SomeEnum < > {
                SomeArr([u64; 7usize])
            }
        };

        assert_eq!(actual.to_string(), expected.to_string());
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

        let actual = expand_custom_enum(
            &FullTypeDeclaration::from_counterpart(&p, &types),
            &HashSet::default(),
            false,
        )?
        .code;

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
            #[FuelsTypesPath("::fuels::types")]
            #[FuelsCorePath("::fuels::core")]
            pub enum EnumLevel3<> {
                El2(self::EnumLevel2)
            }
        };

        assert_eq!(actual.to_string(), expected.to_string());
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

        let actual = expand_custom_struct(
            &FullTypeDeclaration::from_counterpart(&p, &types),
            &HashSet::default(),
            false,
        )?
        .code;

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
            #[FuelsTypesPath("::fuels::types")]
            #[FuelsCorePath("::fuels::core")]
            pub struct Cocktail < > {
                pub long_island: bool,
                pub cosmopolitan: u64,
                pub mojito: u32
            }
        };

        assert_eq!(actual.to_string(), expected.to_string());

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

        let actual = expand_custom_struct(
            &FullTypeDeclaration::from_counterpart(&p, &types),
            &HashSet::default(),
            false,
        )?
        .code;

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
            #[FuelsTypesPath("::fuels::types")]
            #[FuelsCorePath("::fuels::core")]
            pub struct SomeEmptyStruct < > {}
        };

        assert_eq!(actual.to_string(), expected.to_string());

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

        let actual = expand_custom_struct(
            &FullTypeDeclaration::from_counterpart(&p, &types),
            &HashSet::default(),
            false,
        )?
        .code;

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
            #[FuelsTypesPath("::fuels::types")]
            #[FuelsCorePath("::fuels::core")]
            pub struct Cocktail < > {
                pub long_island: self::Shaker,
                pub mojito: u32
            }
        };

        assert_eq!(actual.to_string(), expected.to_string());
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

        let actual = expand_custom_struct(
            &FullTypeDeclaration::from_counterpart(s1, &types),
            &HashSet::default(),
            false,
        )?
        .code;

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
            #[FuelsTypesPath("::fuels::types")]
            #[FuelsCorePath("::fuels::core")]
            pub struct MyStruct1 < > {
                pub x: u64,
                pub y: ::fuels::types::Bits256
            }
        };

        assert_eq!(actual.to_string(), expected.to_string());

        let s2 = types.get(&4).unwrap();

        let actual = expand_custom_struct(
            &FullTypeDeclaration::from_counterpart(s2, &types),
            &HashSet::default(),
            false,
        )?
        .code;

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
            #[FuelsTypesPath("::fuels::types")]
            #[FuelsCorePath("::fuels::core")]
            pub struct MyStruct2 < > {
                pub x: bool,
                pub y: self::MyStruct1
            }
        };

        assert_eq!(actual.to_string(), expected.to_string());

        Ok(())
    }

    #[test]
    fn will_skip_shared_types() {
        // given
        let types = ["struct SomeStruct", "enum SomeEnum"].map(given_a_custom_type);
        let shared_types = HashSet::from_iter(types.iter().take(1).cloned());

        // when
        let generated_code =
            generate_types(types, &shared_types, false).expect("Should have succeeded.");

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
