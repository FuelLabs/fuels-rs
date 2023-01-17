use std::collections::HashSet;

use proc_macro2::{Ident, TokenStream};
use quote::quote;

use fuels_types::errors::Error;
use fuels_types::utils::extract_generic_name;

use crate::code_gen::abi_types::FullTypeDeclaration;
use crate::code_gen::utils::Component;
use crate::utils::ident;

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
        impl<#(#generics: ::fuels::core::Tokenizable + ::fuels::core::Parameterize),*> TryFrom<&[u8]> for self::#ident<#(#generics),*> {
            type Error = ::fuels::types::errors::Error;

            fn try_from(bytes: &[u8]) -> ::std::result::Result<Self, Self::Error> {
                ::fuels::core::try_from_bytes(bytes)
            }
        }
        impl<#(#generics: ::fuels::core::Tokenizable + ::fuels::core::Parameterize),*> TryFrom<&::std::vec::Vec<u8>> for self::#ident<#(#generics),*> {
            type Error = ::fuels::types::errors::Error;

            fn try_from(bytes: &::std::vec::Vec<u8>) -> ::std::result::Result<Self, Self::Error> {
                ::fuels::core::try_from_bytes(&bytes)
            }
        }

        impl<#(#generics: ::fuels::core::Tokenizable + ::fuels::core::Parameterize),*> TryFrom<::std::vec::Vec<u8>> for self::#ident<#(#generics),*> {
            type Error = ::fuels::types::errors::Error;

            fn try_from(bytes: ::std::vec::Vec<u8>) -> ::std::result::Result<Self, Self::Error> {
                ::fuels::core::try_from_bytes(&bytes)
            }
        }
    }
}

/// Transforms components from inside the given `FullTypeDeclaration` into a vector
/// of `Components`. Will fail if there are no components.
pub(crate) fn extract_components(
    type_decl: &FullTypeDeclaration,
    snake_case: bool,
    shared_types: &HashSet<FullTypeDeclaration>,
) -> Result<Vec<Component>, Error> {
    type_decl
        .components
        .iter()
        .map(|component| Component::new(component, snake_case, shared_types))
        .collect()
}

/// Returns a vector of TokenStreams, one for each of the generic parameters
/// used by the given type.
pub(crate) fn extract_generic_parameters(
    type_decl: &FullTypeDeclaration,
) -> Result<Vec<TokenStream>, Error> {
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
    use fuel_abi_types::program_abi::TypeDeclaration;
    use fuels_types::utils::custom_type_name;

    use crate::code_gen::resolved_type::ResolvedType;
    use crate::code_gen::utils::param_type_calls;

    use super::*;

    #[test]
    fn extracts_generic_types() -> anyhow::Result<()> {
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
                "< u8 as :: fuels :: core :: Parameterize > :: param_type ()",
                "< SomeStruct :: < T , K > as :: fuels :: core :: Parameterize > :: param_type ()"
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
