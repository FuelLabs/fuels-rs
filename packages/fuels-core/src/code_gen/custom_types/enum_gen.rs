use super::utils::{
    extract_components, extract_generic_parameters, impl_try_from, param_type_calls, Component,
};
use crate::utils::ident;
use core::result::Result;
use core::result::Result::Ok;
use fuels_types::errors::Error;
use fuels_types::utils::custom_type_name;
use fuels_types::TypeDeclaration;
use proc_macro2::{Ident, TokenStream};
use quote::quote;
use std::collections::HashMap;

/// Returns a TokenStream containing the declaration, `Parameterize`,
/// `Tokenizable` and `TryFrom` implementations for the enum described by the
/// given TypeDeclaration.
pub fn expand_custom_enum(
    type_decl: &TypeDeclaration,
    types: &HashMap<usize, TypeDeclaration>,
) -> Result<TokenStream, Error> {
    let enum_ident = ident(&custom_type_name(&type_decl.type_field)?);

    let components = extract_components(type_decl, types, false)?;
    let generics = extract_generic_parameters(type_decl, types)?;

    let enum_def = enum_decl(&enum_ident, &components, &generics);
    let parameterize_impl = enum_parameterize_impl(&enum_ident, &components, &generics);
    let tokenize_impl = enum_tokenizable_impl(&enum_ident, &components, &generics);
    let try_from = impl_try_from(&enum_ident, &generics);

    Ok(quote! {
        #enum_def

        #parameterize_impl

        #tokenize_impl

        #try_from
    })
}

fn enum_decl(
    enum_ident: &Ident,
    components: &[Component],
    generics: &[TokenStream],
) -> TokenStream {
    let enum_variants = components.iter().map(
        |Component {
             field_name,
             field_type,
         }| {
            let field_type = if field_type.is_unit() {
                quote! {}
            } else {
                field_type.into()
            };

            quote! {
                #field_name(#field_type)
            }
        },
    );

    quote! {
        #[derive(Clone, Debug, Eq, PartialEq)]
        pub enum #enum_ident <#(#generics: ::fuels::core::Tokenizable + ::fuels::core::Parameterize),*> {
            #(#enum_variants),*
        }
    }
}

fn enum_tokenizable_impl(
    enum_ident: &Ident,
    components: &[Component],
    generics: &[TokenStream],
) -> TokenStream {
    let enum_ident_stringified = enum_ident.to_string();

    let match_discriminant_from_token = components.iter().enumerate().map(
        |(
            discriminant,
            Component {
                field_name,
                field_type,
            },
        )| {
            let value = if field_type.is_unit() {
                quote! {}
            } else {
                let field_type: TokenStream = field_type.into();
                quote! { <#field_type>::from_token(variant_token)? }
            };

            let u8_discriminant = discriminant as u8;
            quote! { #u8_discriminant => Ok(Self::#field_name(#value))}
        },
    );

    let match_discriminant_into_token = components.iter().enumerate().map(
        |(
            discriminant,
            Component {
                field_name,
                field_type,
            },
        )| {
            let u8_discriminant = discriminant as u8;
            if field_type.is_unit() {
                quote! { Self::#field_name() => (#u8_discriminant, ().into_token())}
            } else {
                quote! { Self::#field_name(inner) => (#u8_discriminant, inner.into_token())}
            }
        },
    );

    quote! {
            impl<#(#generics: ::fuels::core::Tokenizable + ::fuels::core::Parameterize),*> ::fuels::core::Tokenizable for #enum_ident <#(#generics),*> {
                fn from_token(token: ::fuels::core::Token) -> Result<Self, ::fuels::types::errors::Error>
                where
                    Self: Sized,
                {
                    let gen_err = |msg| {
                        ::fuels::types::errors::Error::InvalidData(format!(
                            "Error while instantiating {} from token! {}", #enum_ident_stringified, msg
                        ))
                    };
                    match token {
                        ::fuels::core::Token::Enum(selector) => {
                            let (discriminant, variant_token, _) = *selector;
                            match discriminant {
                                #(#match_discriminant_from_token,)*
                                _ => Err(gen_err(format!(
                                    "Discriminant {} doesn't point to any of the enums variants.", discriminant
                                ))),
                            }
                        }
                        _ => Err(gen_err(format!(
                            "Given token ({}) is not of the type Token::Enum!", token
                        ))),
                    }
                }

                fn into_token(self) -> ::fuels::core::Token {
                    let (discriminant, token) = match self {
                        #(#match_discriminant_into_token),*
                    };

                    let variants = match <Self as ::fuels::core::Parameterize>::param_type() {
                        ::fuels::types::param_types::ParamType::Enum{variants, ..} => variants,
                        other => panic!("Calling {}::param_type() must return a ParamType::Enum but instead it returned: {:?}", #enum_ident_stringified, other)
                    };

                    ::fuels::core::Token::Enum(Box::new((discriminant, token, variants)))
                }
            }
    }
}

fn enum_parameterize_impl(
    enum_ident: &Ident,
    components: &[Component],
    generics: &[TokenStream],
) -> TokenStream {
    let param_type_calls = param_type_calls(components);
    let enum_ident_stringified = enum_ident.to_string();
    quote! {
        impl<#(#generics: ::fuels::core::Parameterize + ::fuels::core::Tokenizable),*> ::fuels::core::Parameterize for #enum_ident <#(#generics),*> {
            fn param_type() -> ::fuels::types::param_types::ParamType {
                let mut param_types = vec![];
                #(param_types.push(#param_type_calls);)*

                let variants = ::fuels::types::enum_variants::EnumVariants::new(param_types).unwrap_or_else(|_| panic!("{} has no variants which isn't allowed!", #enum_ident_stringified));
                ::fuels::types::param_types::ParamType::Enum{variants, generics: vec![#(#generics::param_type()),*]}
            }
        }
    }
}
