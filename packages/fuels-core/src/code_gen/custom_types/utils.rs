use crate::code_gen::resolved_type::{resolve_type, ResolvedType};
use crate::utils::{ident, safe_ident};
use fuel_abi_types::program_abi::{TypeApplication, TypeDeclaration};
use fuels_types::errors::Error;
use fuels_types::utils::extract_generic_name;
use inflector::Inflector;
use proc_macro2::{Ident, TokenStream};
use quote::quote;
use std::collections::HashMap;

// Represents a component of either a struct(field name) or an enum(variant
// name).
#[derive(Debug)]
pub struct Component {
    pub field_name: Ident,
    pub field_type: ResolvedType,
}

impl Component {
    pub fn new(
        component: &TypeApplication,
        types: &HashMap<usize, TypeDeclaration>,
        snake_case: bool,
    ) -> Result<Component, Error> {
        let field_name = if snake_case {
            component.name.to_snake_case()
        } else {
            component.name.to_owned()
        };

        Ok(Component {
            field_name: safe_ident(&field_name),
            field_type: resolve_type(component, types)?,
        })
    }
}

/// These TryFrom implementations improve devx by enabling users to easily
/// construct contract types from bytes. These are generated due to the orphan
/// rule prohibiting us from specifying an implementation for all possible
/// types.
///
/// # Arguments
///
/// * `ident`: The name of the struct/enum for which we're generating the code.
/// * `generics`: The generic types of the struct/enum -- i.e. For MyStruct<T,
///               K> it would be ['T', 'K']
///
/// returns: a TokenStream containing the three TryFrom implementations for a
/// &[u8], &Vec<u8> and a Vec<u8>
pub(crate) fn impl_try_from(ident: &Ident, generics: &[TokenStream]) -> TokenStream {
    quote! {
        impl<#(#generics: Tokenizable + Parameterize),*> TryFrom<&[u8]> for #ident<#(#generics),*> {
            type Error = SDKError;

            fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
                try_from_bytes(bytes)
            }
        }
        impl<#(#generics: Tokenizable + Parameterize),*> TryFrom<&Vec<u8>> for #ident<#(#generics),*> {
            type Error = SDKError;

            fn try_from(bytes: &Vec<u8>) -> Result<Self, Self::Error> {
                try_from_bytes(&bytes)
            }
        }

        impl<#(#generics: Tokenizable + Parameterize),*> TryFrom<Vec<u8>> for #ident<#(#generics),*> {
            type Error = SDKError;

            fn try_from(bytes: Vec<u8>) -> Result<Self, Self::Error> {
                try_from_bytes(&bytes)
            }
        }
    }
}

/// Transforms components from inside the given `TypeDeclaration` into a vector
/// of `Components`. Will fail if there are no components.
pub(crate) fn extract_components(
    type_decl: &TypeDeclaration,
    types: &HashMap<usize, TypeDeclaration>,
    snake_case: bool,
) -> Result<Vec<Component>, Error> {
    type_decl
        .components
        .as_ref()
        .unwrap_or(&vec![])
        .iter()
        .map(|component| Component::new(component, types, snake_case))
        .collect()
}

/// Returns a vector of TokenStreams, one for each of the generic parameters
/// used by the given type.
pub fn extract_generic_parameters(
    type_decl: &TypeDeclaration,
    types: &HashMap<usize, TypeDeclaration>,
) -> Result<Vec<TokenStream>, Error> {
    type_decl
        .type_parameters
        .iter()
        .flatten()
        .map(|id| types.get(id).unwrap())
        .map(|decl| {
            let name = extract_generic_name(&decl.type_field).unwrap_or_else(|| {
                panic!("Type parameters should only contain ids of generic types!")
            });
            let generic = ident(&name);
            Ok(quote! {#generic})
        })
        .collect()
}

/// Returns TokenStreams representing calls to `Parameterize::param_type` for
/// all given Components. Makes sure to properly handle calls when generics are
/// involved.
pub fn param_type_calls(field_entries: &[Component]) -> Vec<TokenStream> {
    field_entries
        .iter()
        .map(|Component { field_type, .. }| single_param_type_call(field_type))
        .collect()
}

/// Returns a TokenStream representing the call to `Parameterize::param_type` for
/// the given ResolvedType. Makes sure to properly handle calls when generics are
/// involved.
pub fn single_param_type_call(field_type: &ResolvedType) -> TokenStream {
    let type_name = &field_type.type_name;
    let parameters = field_type
        .generic_params
        .iter()
        .map(TokenStream::from)
        .collect::<Vec<_>>();
    if parameters.is_empty() {
        quote! { <#type_name>::param_type() }
    } else {
        quote! { #type_name::<#(#parameters),*>::param_type() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fuels_types::utils::custom_type_name;

    #[test]
    fn component_name_is_snake_case_when_requested() -> anyhow::Result<()> {
        let type_application = TypeApplication {
            name: "SomeNameHere".to_string(),
            type_id: 0,
            type_arguments: None,
        };

        let types = HashMap::from([(
            0,
            TypeDeclaration {
                type_id: 0,
                type_field: "()".to_string(),
                components: None,
                type_parameters: None,
            },
        )]);

        let component = Component::new(&type_application, &types, true)?;

        assert_eq!(component.field_name, ident("some_name_here"));

        Ok(())
    }
    #[test]
    fn extracts_generic_types() -> anyhow::Result<()> {
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

        let generics = extract_generic_parameters(&declaration, &types)?;

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
                "< u8 > :: param_type ()",
                "SomeStruct :: < T , K > :: param_type ()"
            ]
        )
    }
    #[test]
    fn can_extract_struct_name() -> anyhow::Result<()> {
        let declaration = TypeDeclaration {
            type_id: 0,
            type_field: "struct SomeName".to_string(),
            components: None,
            type_parameters: None,
        };

        let struct_name = custom_type_name(&declaration.type_field)?;

        assert_eq!(struct_name, "SomeName");

        Ok(())
    }

    #[test]
    fn can_extract_enum_name() -> anyhow::Result<()> {
        let declaration = TypeDeclaration {
            type_id: 0,
            type_field: "enum SomeEnumName".to_string(),
            components: None,
            type_parameters: None,
        };

        let struct_name = custom_type_name(&declaration.type_field)?;

        assert_eq!(struct_name, "SomeEnumName");

        Ok(())
    }
}
