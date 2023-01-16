use std::collections::HashSet;

use fuels_types::{errors::Error, utils::custom_type_name};
use proc_macro2::{Ident, TokenStream};
use quote::quote;

use crate::{
    code_gen::{
        abi_types::FullTypeDeclaration,
        custom_types::utils::{extract_components, extract_generic_parameters, impl_try_from},
        generated_code::GeneratedCode,
        type_path::TypePath,
        utils::{param_type_calls, Component},
    },
    utils::ident,
};

/// Returns a TokenStream containing the declaration, `Parameterize`,
/// `Tokenizable` and `TryFrom` implementations for the struct described by the
/// given TypeDeclaration.
pub(crate) fn expand_custom_struct(
    type_decl: &FullTypeDeclaration,
    shared_types: &HashSet<FullTypeDeclaration>,
) -> Result<GeneratedCode, Error> {
    let struct_name = custom_type_name(&type_decl.type_field)?;
    let struct_ident = ident(&struct_name);

    let components = extract_components(type_decl, true, shared_types)?;
    let generic_parameters = extract_generic_parameters(type_decl)?;

    let struct_decl = struct_decl(&struct_ident, &components, &generic_parameters);

    let parameterized_impl =
        struct_parameterized_impl(&components, &struct_ident, &generic_parameters);

    let tokenizable_impl = struct_tokenizable_impl(&struct_ident, &components, &generic_parameters);

    let try_from_impl = impl_try_from(&struct_ident, &generic_parameters);

    let code = quote! {
        #struct_decl

        #parameterized_impl

        #tokenizable_impl

        #try_from_impl
    };
    Ok(GeneratedCode {
        code,
        usable_types: HashSet::from([
            TypePath::new(&struct_name).expect("Struct name is not empty!")
        ]),
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
        pub struct #struct_ident <#(#generic_parameters: ::fuels::core::traits::Tokenizable + ::fuels::core::traits::Parameterize, )*> {
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
                #field_name: ::fuels::core::traits::Tokenizable::from_token(next_token()?)?
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
        impl <#(#generic_parameters: ::fuels::core::traits::Tokenizable + ::fuels::core::traits::Parameterize, )*> ::fuels::core::traits::Tokenizable for self::#struct_ident <#(#generic_parameters, )*> {
            fn into_token(self) -> ::fuels::types::Token {
                let tokens = [#(#into_token_calls),*].to_vec();
                ::fuels::types::Token::Struct(tokens)
            }

            fn from_token(token: ::fuels::types::Token)  -> ::std::result::Result<Self, ::fuels::types::errors::Error> {
                match token {
                    ::fuels::types::Token::Struct(tokens) => {
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
        impl <#(#generic_parameters: ::fuels::core::traits::Parameterize + ::fuels::core::traits::Tokenizable),*> ::fuels::core::traits::Parameterize for self::#struct_ident <#(#generic_parameters),*> {
            fn param_type() -> ::fuels::types::param_types::ParamType {
                let types = [#(#field_name_param_type),*].to_vec();
                ::fuels::types::param_types::ParamType::Struct{
                    name: #struct_name_str.to_string(),
                    fields: types,
                    generics: [#(#generic_parameters::param_type()),*].to_vec()
                }
            }
        }
    }
}
