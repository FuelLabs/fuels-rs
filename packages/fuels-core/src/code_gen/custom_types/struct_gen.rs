use super::utils::{
    extract_components, extract_generic_parameters, impl_try_from, param_type_calls, Component,
};
use crate::code_gen::full_abi_types::FullTypeDeclaration;
use crate::utils::ident;
use core::result::Result;
use core::result::Result::Ok;
use fuels_types::errors::Error;
use fuels_types::utils::custom_type_name;
use proc_macro2::{Ident, TokenStream};
use quote::quote;

/// Returns a TokenStream containing the declaration, `Parameterize`,
/// `Tokenizable` and `TryFrom` implementations for the struct described by the
/// given TypeDeclaration.
pub fn expand_custom_struct(
    type_decl: &FullTypeDeclaration,
    common_types: &[FullTypeDeclaration],
) -> Result<TokenStream, Error> {
    let struct_ident = ident(&custom_type_name(&type_decl.type_field)?);

    let components = extract_components(type_decl, true, common_types)?;
    let generic_parameters = extract_generic_parameters(type_decl)?;

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
        pub struct #struct_ident <#(#generic_parameters: ::fuels::core::Tokenizable + ::fuels::core::Parameterize, )*> {
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
        .map(|Component { field_name, .. }| {
            quote! {
                #field_name: ::fuels::core::Tokenizable::from_token(next_token()?)?
            }
        })
        .collect::<Vec<_>>();

    let into_token_calls = components
        .iter()
        .map(|Component { field_name, .. }| {
            quote! {self.#field_name.into_token()}
        })
        .collect::<Vec<_>>();

    quote! {
        impl <#(#generic_parameters: ::fuels::core::Tokenizable + ::fuels::core::Parameterize, )*> ::fuels::core::Tokenizable for self::#struct_ident <#(#generic_parameters, )*> {
            fn from_token(token: ::fuels::core::Token)  -> ::std::result::Result<Self, ::fuels::types::errors::Error> {
                match token {
                    ::fuels::core::Token::Struct(tokens) => {
                        let mut tokens_iter = tokens.into_iter();
                        let mut next_token = move || { tokens_iter
                            .next()
                            .ok_or_else(|| { ::fuels::types::errors::Error::InstantiationError(format!("Ran out of tokens before '{}' has finished construction!", #struct_name_str)) })
                        };
                        ::std::result::Result::Ok(Self { #( #from_token_calls, )* })
                    },
                    other => ::std::result::Result::Err(::fuels::types::errors::Error::InstantiationError(format!("Error while constructing '{}'. Expected token of type Token::Struct, got {:?}", #struct_name_str, other))),
                }
            }

            fn into_token(self) -> ::fuels::core::Token {
                let mut tokens = ::std::vec::Vec::new();
                #( tokens.push(#into_token_calls); )*
                ::fuels::core::Token::Struct(tokens)
            }

        }
    }
}

fn struct_parameterized_impl(
    components: &[Component],
    struct_ident: &Ident,
    generic_parameters: &[TokenStream],
) -> TokenStream {
    let param_type_calls = param_type_calls(components);
    quote! {
        impl <#(#generic_parameters: ::fuels::core::Parameterize + ::fuels::core::Tokenizable),*> ::fuels::core::Parameterize for self::#struct_ident <#(#generic_parameters),*> {
            fn param_type() -> ::fuels::types::param_types::ParamType {
                let mut types = ::std::vec::Vec::new();
                #( types.push(#param_type_calls); )*
                ::fuels::types::param_types::ParamType::Struct{fields: types, generics: vec![#(#generic_parameters::param_type()),*]}
            }
        }
    }
}
