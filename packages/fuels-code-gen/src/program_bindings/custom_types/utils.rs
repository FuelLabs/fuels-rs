use fuel_abi_types::{
    abi::full_program::FullTypeDeclaration,
    utils::{self, extract_generic_name},
};
use proc_macro2::Ident;

/// Returns a vector of TokenStreams, one for each of the generic parameters
/// used by the given type.
pub(crate) fn extract_generic_parameters(type_decl: &FullTypeDeclaration) -> Vec<Ident> {
    type_decl
        .type_parameters
        .iter()
        .map(|decl| {
            let name = extract_generic_name(&decl.type_field).unwrap_or_else(|| {
                panic!("Type parameters should only contain ids of generic types!")
            });
            utils::ident(&name)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use fuel_abi_types::{
        abi::{full_program::FullTypeApplication, program::TypeDeclaration},
        utils::{extract_custom_type_name, TypePath},
    };
    use pretty_assertions::assert_eq;
    use quote::quote;

    use super::*;
    use crate::{error::Result, program_bindings::utils::Components};

    #[test]
    fn extracts_generic_types() -> Result<()> {
        // given
        let declaration = TypeDeclaration {
            type_id: 0,
            type_field: "".to_string(),
            components: None,
            type_parameters: Some(vec![1, 2]),
        };
        let generic_1 = TypeDeclaration {
            type_id: 1,
            type_field: "generic T".to_string(),
            components: None,
            type_parameters: None,
        };

        let generic_2 = TypeDeclaration {
            type_id: 2,
            type_field: "generic K".to_string(),
            components: None,
            type_parameters: None,
        };

        let types = [generic_1, generic_2]
            .map(|decl| (decl.type_id, decl))
            .into_iter()
            .collect();

        // when
        let generics = extract_generic_parameters(&FullTypeDeclaration::from_counterpart(
            &declaration,
            &types,
        ));

        // then
        let stringified_generics = generics
            .into_iter()
            .map(|generic| generic.to_string())
            .collect::<Vec<_>>();

        assert_eq!(stringified_generics, vec!["T", "K"]);

        Ok(())
    }

    #[test]
    fn param_type_calls_correctly_generated() {
        // arrange
        let type_applications = vec![
            FullTypeApplication {
                name: "unimportant".to_string(),
                type_decl: FullTypeDeclaration {
                    type_field: "u8".to_string(),
                    components: vec![],
                    type_parameters: vec![],
                },
                type_arguments: vec![],
            },
            FullTypeApplication {
                name: "unimportant".to_string(),
                type_decl: FullTypeDeclaration {
                    type_field: "struct SomeStruct".to_string(),
                    components: vec![],
                    type_parameters: vec![
                        FullTypeDeclaration {
                            type_field: "generic T".to_string(),
                            components: vec![],
                            type_parameters: vec![],
                        },
                        FullTypeDeclaration {
                            type_field: "generic K".to_string(),
                            components: vec![],
                            type_parameters: vec![],
                        },
                    ],
                },
                type_arguments: vec![
                    FullTypeApplication {
                        name: "unimportant".to_string(),
                        type_decl: FullTypeDeclaration {
                            type_field: "u8".to_string(),
                            components: vec![],
                            type_parameters: vec![],
                        },
                        type_arguments: vec![],
                    },
                    FullTypeApplication {
                        name: "unimportant".to_string(),
                        type_decl: FullTypeDeclaration {
                            type_field: "u16".to_string(),
                            components: vec![],
                            type_parameters: vec![],
                        },
                        type_arguments: vec![],
                    },
                ],
            },
        ];

        // act
        let param_type_calls = Components::new(&type_applications, true, TypePath::default())
            .unwrap()
            .param_type_calls();

        // assert
        let stringified_result = param_type_calls
            .into_iter()
            .map(|stream| stream.to_string())
            .collect::<Vec<_>>();

        let expected = vec![
            quote! { <::core::primitive::u8 as :: fuels::core::traits::Parameterize>::param_type() }.to_string(),
            quote! { <self::SomeStruct<::core::primitive::u8, ::core::primitive::u16> as ::fuels::core::traits::Parameterize>::param_type() }.to_string(),
        ];
        assert_eq!(stringified_result, expected);
    }

    #[test]
    fn can_extract_struct_name() {
        let declaration = TypeDeclaration {
            type_id: 0,
            type_field: "struct SomeName".to_string(),
            components: None,
            type_parameters: None,
        };

        let struct_name = extract_custom_type_name(&declaration.type_field).unwrap();

        assert_eq!(struct_name, "SomeName");
    }

    #[test]
    fn can_extract_enum_name() {
        let declaration = TypeDeclaration {
            type_id: 0,
            type_field: "enum SomeEnumName".to_string(),
            components: None,
            type_parameters: None,
        };

        let struct_name = extract_custom_type_name(&declaration.type_field).unwrap();

        assert_eq!(struct_name, "SomeEnumName");
    }
}
