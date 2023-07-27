use fuel_abi_types::{
    abi::full_program::FullTypeDeclaration,
    utils::{extract_generic_name, ident, TypePath},
};
use proc_macro2::TokenStream;
use quote::quote;

use crate::{error::Result, program_bindings::utils::Component};

/// Transforms components from inside the given `FullTypeDeclaration` into a vector
/// of `Components`. Will fail if there are no components.
pub(crate) fn extract_components(
    type_decl: &FullTypeDeclaration,
    snake_case: bool,
    mod_name: &TypePath,
) -> Result<Vec<Component>> {
    type_decl
        .components
        .iter()
        .map(|component| Component::new(component, snake_case, mod_name.clone()))
        .collect()
}

/// Returns a vector of TokenStreams, one for each of the generic parameters
/// used by the given type.
pub(crate) fn extract_generic_parameters(
    type_decl: &FullTypeDeclaration,
) -> Result<Vec<TokenStream>> {
    type_decl
        .type_parameters
        .iter()
        .map(|decl| {
            let name = extract_generic_name(&decl.type_field).unwrap_or_else(|| {
                panic!("Type parameters should only contain ids of generic types!")
            });
            let generic = ident(&name);
            Ok(quote! {#generic})
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use fuel_abi_types::{abi::program::TypeDeclaration, utils::extract_custom_type_name};

    use super::*;
    use crate::program_bindings::{resolved_type::ResolvedType, utils::param_type_calls};

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
        ))?;

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
        let components = vec![
            Component {
                field_name: ident("a"),
                field_type: ResolvedType {
                    type_name: quote! {u8},
                    generic_params: vec![],
                },
            },
            Component {
                field_name: ident("b"),
                field_type: ResolvedType {
                    type_name: quote! {SomeStruct},
                    generic_params: vec![
                        ResolvedType {
                            type_name: quote! {T},
                            generic_params: vec![],
                        },
                        ResolvedType {
                            type_name: quote! {K},
                            generic_params: vec![],
                        },
                    ],
                },
            },
        ];

        // act
        let result = param_type_calls(&components);

        // assert
        let stringified_result = result
            .into_iter()
            .map(|stream| stream.to_string())
            .collect::<Vec<_>>();
        assert_eq!(
            stringified_result,
            vec![
                "< u8 as :: fuels :: core :: traits :: Parameterize > :: param_type ()",
                "< SomeStruct :: < T , K > as :: fuels :: core :: traits :: Parameterize > :: param_type ()"
            ]
        )
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
