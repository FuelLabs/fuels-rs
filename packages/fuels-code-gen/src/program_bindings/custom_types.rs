use std::collections::{HashMap, HashSet};

use fuel_abi_types::abi::full_program::{FullLoggedType, FullTypeDeclaration};
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
pub(crate) mod utils;

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
///   contracts/scripts/predicates and thus generated elsewhere.
pub(crate) fn generate_types<'a>(
    types: impl IntoIterator<Item = &'a FullTypeDeclaration>,
    shared_types: &HashSet<FullTypeDeclaration>,
    logged_types: impl IntoIterator<Item = &'a FullLoggedType>,
    no_std: bool,
) -> Result<GeneratedCode> {
    let log_ids: HashMap<_, _> = logged_types
        .into_iter()
        .map(|l| (l.application.type_decl.type_field.clone(), l.log_id.clone()))
        .collect();

    types
        .into_iter()
        .filter(|ttype| !should_skip_codegen(ttype))
        .map(|ttype: &FullTypeDeclaration| {
            let log_id = log_ids.get(&ttype.type_field);
            if shared_types.contains(ttype) {
                reexport_the_shared_type(ttype, no_std)
            } else if ttype.is_struct_type() {
                expand_custom_struct(ttype, no_std, log_id)
            } else {
                expand_custom_enum(ttype, no_std, log_id)
            }
        })
        .fold_ok(GeneratedCode::default(), |acc, generated_code| {
            acc.merge(generated_code)
        })
}

/// Instead of generating bindings for `ttype` this fn will just generate a `pub use` pointing to
/// the already generated equivalent shared type.
fn reexport_the_shared_type(ttype: &FullTypeDeclaration, no_std: bool) -> Result<GeneratedCode> {
    // e.g. some_library::another_mod::SomeStruct
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
pub fn should_skip_codegen(type_decl: &FullTypeDeclaration) -> bool {
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
        TypePath::new("std::vec::RawVec").expect(msg),
        TypePath::new("std::bytes::RawBytes").expect(msg),
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

    use fuel_abi_types::abi::unified_program::{UnifiedTypeApplication, UnifiedTypeDeclaration};
    use pretty_assertions::assert_eq;
    use quote::quote;

    use super::*;

    #[test]
    fn test_expand_custom_enum() -> Result<()> {
        let p = UnifiedTypeDeclaration {
            type_id: 0,
            type_field: String::from("enum MatchaTea"),
            components: Some(vec![
                UnifiedTypeApplication {
                    name: String::from("LongIsland"),
                    type_id: 1,
                    ..Default::default()
                },
                UnifiedTypeApplication {
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
                UnifiedTypeDeclaration {
                    type_id: 1,
                    type_field: String::from("u64"),
                    ..Default::default()
                },
            ),
            (
                2,
                UnifiedTypeDeclaration {
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
            false,
            None,
        )?;

        let expected = quote! {
            #[allow(clippy::enum_variant_names)]
            #[derive(
                Clone,
                Debug,
                Eq,
                PartialEq,
                ::fuels::macros::Parameterize,
                ::fuels::macros::Tokenizable,
                ::fuels::macros::TryFrom,
            )]
            pub enum MatchaTea {
                LongIsland(::core::primitive::u64),
                MoscowMule(::core::primitive::bool),
            }
        };

        assert_eq!(actual.code().to_string(), expected.to_string());
        Ok(())
    }

    #[test]
    fn test_enum_with_no_variants_cannot_be_constructed() -> Result<()> {
        let p = UnifiedTypeDeclaration {
            type_id: 0,
            type_field: "enum SomeEmptyEnum".to_string(),
            components: Some(vec![]),
            ..Default::default()
        };
        let types = [(0, p.clone())].into_iter().collect::<HashMap<_, _>>();

        expand_custom_enum(
            &FullTypeDeclaration::from_counterpart(&p, &types),
            false,
            None,
        )
        .expect_err("Was able to construct an enum without variants");

        Ok(())
    }

    #[test]
    fn test_expand_struct_inside_enum() -> Result<()> {
        let inner_struct = UnifiedTypeApplication {
            name: String::from("Infrastructure"),
            type_id: 1,
            ..Default::default()
        };
        let enum_components = vec![
            inner_struct,
            UnifiedTypeApplication {
                name: "Service".to_string(),
                type_id: 2,
                ..Default::default()
            },
        ];
        let p = UnifiedTypeDeclaration {
            type_id: 0,
            type_field: String::from("enum Amsterdam"),
            components: Some(enum_components),
            ..Default::default()
        };

        let types = [
            (0, p.clone()),
            (
                1,
                UnifiedTypeDeclaration {
                    type_id: 1,
                    type_field: String::from("struct Building"),
                    components: Some(vec![]),
                    ..Default::default()
                },
            ),
            (
                2,
                UnifiedTypeDeclaration {
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
            false,
            None,
        )?;

        let expected = quote! {
            #[allow(clippy::enum_variant_names)]
            #[derive(
                Clone,
                Debug,
                Eq,
                PartialEq,
                ::fuels::macros::Parameterize,
                ::fuels::macros::Tokenizable,
                ::fuels::macros::TryFrom,
            )]
            pub enum Amsterdam {
                Infrastructure(self::Building),
                Service(::core::primitive::u32),
            }
        };

        assert_eq!(actual.code().to_string(), expected.to_string());
        Ok(())
    }

    #[test]
    fn test_expand_array_inside_enum() -> Result<()> {
        let enum_components = vec![UnifiedTypeApplication {
            name: "SomeArr".to_string(),
            type_id: 1,
            ..Default::default()
        }];
        let p = UnifiedTypeDeclaration {
            type_id: 0,
            type_field: String::from("enum SomeEnum"),
            components: Some(enum_components),
            ..Default::default()
        };
        let types = [
            (0, p.clone()),
            (
                1,
                UnifiedTypeDeclaration {
                    type_id: 1,
                    type_field: "[u64; 7]".to_string(),
                    components: Some(vec![UnifiedTypeApplication {
                        type_id: 2,
                        ..Default::default()
                    }]),
                    ..Default::default()
                },
            ),
            (
                2,
                UnifiedTypeDeclaration {
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
            false,
            None,
        )?;

        let expected = quote! {
            #[allow(clippy::enum_variant_names)]
            #[derive(
                Clone,
                Debug,
                Eq,
                PartialEq,
                ::fuels::macros::Parameterize,
                ::fuels::macros::Tokenizable,
                ::fuels::macros::TryFrom,
            )]
            pub enum SomeEnum {
                SomeArr([::core::primitive::u64; 7usize]),
            }
        };

        assert_eq!(actual.code().to_string(), expected.to_string());
        Ok(())
    }

    #[test]
    fn test_expand_custom_enum_with_enum() -> Result<()> {
        let p = UnifiedTypeDeclaration {
            type_id: 3,
            type_field: String::from("enum EnumLevel3"),
            components: Some(vec![UnifiedTypeApplication {
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
                UnifiedTypeDeclaration {
                    type_id: 2,
                    type_field: String::from("enum EnumLevel2"),
                    components: Some(vec![UnifiedTypeApplication {
                        name: String::from("El1"),
                        type_id: 1,
                        ..Default::default()
                    }]),
                    ..Default::default()
                },
            ),
            (
                1,
                UnifiedTypeDeclaration {
                    type_id: 1,
                    type_field: String::from("enum EnumLevel1"),
                    components: Some(vec![UnifiedTypeApplication {
                        name: String::from("Num"),
                        type_id: 0,
                        ..Default::default()
                    }]),
                    ..Default::default()
                },
            ),
            (
                0,
                UnifiedTypeDeclaration {
                    type_id: 0,
                    type_field: String::from("u32"),
                    ..Default::default()
                },
            ),
        ]
        .into_iter()
        .collect::<HashMap<_, _>>();

        let log_id = "42".to_string();
        let actual = expand_custom_enum(
            &FullTypeDeclaration::from_counterpart(&p, &types),
            false,
            Some(&log_id),
        )?;

        let expected = quote! {
            #[allow(clippy::enum_variant_names)]
            #[derive(
                Clone,
                Debug,
                Eq,
                PartialEq,
                ::fuels::macros::Parameterize,
                ::fuels::macros::Tokenizable,
                ::fuels::macros::TryFrom,
            )]
            pub enum EnumLevel3 {
                El2(self::EnumLevel2),
            }

            impl ::fuels::core::codec::Log for EnumLevel3 {
                fn log_id() -> &'static str {
                    "42"
                }

                fn log_id_u64() -> u64 {
                    42u64
                }
            }
        };

        assert_eq!(actual.code().to_string(), expected.to_string());
        Ok(())
    }

    #[test]
    fn test_expand_custom_struct() -> Result<()> {
        let p = UnifiedTypeDeclaration {
            type_field: String::from("struct Cocktail"),
            components: Some(vec![
                UnifiedTypeApplication {
                    name: String::from("long_island"),
                    type_id: 1,
                    ..Default::default()
                },
                UnifiedTypeApplication {
                    name: String::from("cosmopolitan"),
                    type_id: 2,
                    ..Default::default()
                },
                UnifiedTypeApplication {
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
                UnifiedTypeDeclaration {
                    type_id: 1,
                    type_field: String::from("bool"),
                    ..Default::default()
                },
            ),
            (
                2,
                UnifiedTypeDeclaration {
                    type_id: 2,
                    type_field: String::from("u64"),
                    ..Default::default()
                },
            ),
            (
                3,
                UnifiedTypeDeclaration {
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
            false,
            None,
        )?;

        let expected = quote! {
            #[derive(
                Clone,
                Debug,
                Eq,
                PartialEq,
                ::fuels::macros::Parameterize,
                ::fuels::macros::Tokenizable,
                ::fuels::macros::TryFrom,
            )]
            pub struct Cocktail {
                pub long_island: ::core::primitive::bool,
                pub cosmopolitan: ::core::primitive::u64,
                pub mojito: ::core::primitive::u32,
            }
            impl Cocktail {
                pub fn new(
                    long_island: ::core::primitive::bool,
                    cosmopolitan: ::core::primitive::u64,
                    mojito: ::core::primitive::u32,
                ) -> Self {
                    Self {
                        long_island,
                        cosmopolitan,
                        mojito,
                    }
                }
            }
        };

        assert_eq!(actual.code().to_string(), expected.to_string());

        Ok(())
    }

    #[test]
    fn test_struct_with_no_fields_can_be_constructed() -> Result<()> {
        let p = UnifiedTypeDeclaration {
            type_id: 0,
            type_field: "struct SomeEmptyStruct".to_string(),
            components: Some(vec![]),
            ..Default::default()
        };
        let types = [(0, p.clone())].into_iter().collect::<HashMap<_, _>>();

        let actual = expand_custom_struct(
            &FullTypeDeclaration::from_counterpart(&p, &types),
            false,
            None,
        )?;

        let expected = quote! {
            #[derive(
                Clone,
                Debug,
                Eq,
                PartialEq,
                ::core::default::Default,
                ::fuels::macros::Parameterize,
                ::fuels::macros::Tokenizable,
                ::fuels::macros::TryFrom,
            )]
            pub struct SomeEmptyStruct {}
            impl SomeEmptyStruct {
                pub fn new() -> Self {
                    Self {}
                }
            }
        };

        assert_eq!(actual.code().to_string(), expected.to_string());

        Ok(())
    }

    #[test]
    fn test_expand_custom_struct_with_struct() -> Result<()> {
        let p = UnifiedTypeDeclaration {
            type_id: 0,
            type_field: String::from("struct Cocktail"),
            components: Some(vec![
                UnifiedTypeApplication {
                    name: String::from("long_island"),
                    type_id: 1,
                    ..Default::default()
                },
                UnifiedTypeApplication {
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
                UnifiedTypeDeclaration {
                    type_id: 1,
                    type_field: String::from("struct Shaker"),
                    components: Some(vec![]),
                    ..Default::default()
                },
            ),
            (
                2,
                UnifiedTypeDeclaration {
                    type_id: 2,
                    type_field: String::from("u32"),
                    ..Default::default()
                },
            ),
        ]
        .into_iter()
        .collect::<HashMap<_, _>>();

        let log_id = "13".to_string();
        let actual = expand_custom_struct(
            &FullTypeDeclaration::from_counterpart(&p, &types),
            false,
            Some(&log_id),
        )?;

        let expected = quote! {
            #[derive(
                Clone,
                Debug,
                Eq,
                PartialEq,
                ::fuels::macros::Parameterize,
                ::fuels::macros::Tokenizable,
                ::fuels::macros::TryFrom,
            )]
            pub struct Cocktail {
                pub long_island: self::Shaker,
                pub mojito: ::core::primitive::u32,
            }
            impl Cocktail {
                pub fn new(long_island: self::Shaker, mojito: ::core::primitive::u32,) -> Self {
                    Self {
                        long_island,
                        mojito,
                    }
                }
            }

            impl ::fuels::core::codec::Log for Cocktail {
                fn log_id() -> &'static str {
                    "13"
                }

                fn log_id_u64() -> u64 {
                    13u64
                }
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
            alias_of: None,
        };
        let shared_types = HashSet::from([type_decl.clone()]);

        // when
        let generated_code = generate_types(&[type_decl], &shared_types, [], false).unwrap();

        // then
        let expected_code = quote! {
            #[allow(clippy::too_many_arguments)]
            #[allow(clippy::disallowed_names)]
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

                use ::std::{string::ToString, format, vec, default::Default};
                pub use super::super::shared_types::some_shared_lib::SharedStruct;
            }
        };

        assert_eq!(generated_code.code().to_string(), expected_code.to_string());
    }
}
