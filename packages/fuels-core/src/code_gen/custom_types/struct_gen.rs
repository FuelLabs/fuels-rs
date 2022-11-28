use super::utils::{
    extract_components, extract_generic_parameters, impl_try_from, param_type_calls, Component,
};
use crate::utils::ident;
use fuels_types::{errors::Error, utils::custom_type_name, TypeDeclaration};
use proc_macro2::{Ident, TokenStream};
use quote::quote;
use std::collections::HashMap;

/// Returns a TokenStream containing the declaration, `Parameterize`,
/// `Tokenizable` and `TryFrom` implementations for the struct described by the
/// given TypeDeclaration.
pub fn expand_custom_struct(
    type_decl: &TypeDeclaration,
    types: &HashMap<usize, TypeDeclaration>,
) -> Result<TokenStream, Error> {
    let struct_ident = ident(&custom_type_name(&type_decl.type_field)?);

    let components = extract_components(type_decl, types, true)?;
    let generic_parameters = extract_generic_parameters(type_decl, types)?;

    let struct_decl = struct_decl(&struct_ident, &components, &generic_parameters);

    let parameterized_impl =
        struct_parameterized_impl(&components, &struct_ident, &generic_parameters);

    let tokenizable_impl = struct_tokenizable_impl(&struct_ident, &components, &generic_parameters);

    let try_from_impl = impl_try_from(&struct_ident, &generic_parameters);

    Ok(quote! {
        #struct_decl

        #parameterized_impl

        #tokenizable_impl

        #try_from_impl
    })
}

fn struct_decl(
    struct_ident: &Ident,
    components: &[Component],
    generic_parameters: &Vec<TokenStream>,
) -> TokenStream {
    let fields = components.iter().map(
        |Component {
             field_name,
             field_type,
         }| {
            let field_type: TokenStream = field_type.into();
            quote! { pub #field_name: #field_type }
        },
    );

    quote! {
        #[derive(Clone, Debug, Eq, PartialEq)]
        pub struct #struct_ident <#(#generic_parameters: Tokenizable + Parameterize, )*> {
            #(#fields),*
        }
    }
}

fn struct_tokenizable_impl(
    struct_ident: &Ident,
    components: &[Component],
    generic_parameters: &Vec<TokenStream>,
) -> TokenStream {
    let struct_name_str = struct_ident.to_string();
    let from_token_calls = components
        .iter()
        .map(
            |Component {
                 field_name,
                 field_type,
             }| {
                let resolved: TokenStream = field_type.into();
                quote! {
                    #field_name: <#resolved>::from_token(next_token()?)?
                }
            },
        )
        .collect::<Vec<_>>();

    let into_token_calls = components
        .iter()
        .map(|Component { field_name, .. }| {
            quote! {self.#field_name.into_token()}
        })
        .collect::<Vec<_>>();

    quote! {
        impl <#(#generic_parameters: Tokenizable + Parameterize, )*> Tokenizable for #struct_ident <#(#generic_parameters, )*> {
            fn into_token(self) -> Token {
                let tokens = [#(#into_token_calls),*].to_vec();
                Token::Struct(tokens)
            }

            fn from_token(token: Token)  -> Result<Self, SDKError> {
                match token {
                    Token::Struct(tokens) => {
                        let mut tokens_iter = tokens.into_iter();
                        let mut next_token = move || { tokens_iter
                            .next()
                            .ok_or_else(|| { SDKError::InstantiationError(format!("Ran out of tokens before '{}' has finished construction!", #struct_name_str)) })
                        };
                        Ok(Self { #( #from_token_calls, )* })
                    },
                    other => Err(SDKError::InstantiationError(format!("Error while constructing '{}'. Expected token of type Token::Struct, got {:?}", #struct_name_str, other))),
                }
            }
        }
    }
}

fn struct_parameterized_impl(
    components: &[Component],
    struct_ident: &Ident,
    generic_parameters: &[TokenStream],
) -> TokenStream {
    let field_name_param_type = components
        .iter()
        .map(|component| {
            let field_name = component.field_name.to_string();
            quote! {#field_name.to_string()}
        })
        .zip(param_type_calls(components))
        .map(|(field_name, param_type_call)| {
            quote! {(#field_name, #param_type_call)}
        });
    let struct_name_str = struct_ident.to_string();
    quote! {
        impl <#(#generic_parameters: Parameterize + Tokenizable),*> Parameterize for #struct_ident <#(#generic_parameters),*> {
            fn param_type() -> ParamType {
                let types = [#(#field_name_param_type),*].to_vec();
                ParamType::Struct{
                    name: #struct_name_str.to_string(),
                    fields: types,
                    generics: [#(#generic_parameters::param_type()),*].to_vec()
                }
            }
        }
    }
}
