use super::utils::{
    extract_components, extract_custom_type_name_from_abi_property, extract_generic_parameters,
    impl_try_from, param_type_calls, Component,
};
use core::result::Result;
use core::result::Result::Ok;
use fuels_types::errors::Error;
use fuels_types::param_types::ParamType;
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
    let enum_ident = extract_custom_type_name_from_abi_property(type_decl)?;

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
            let field_type = if let ParamType::Unit = field_type.param_type {
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
        pub enum #enum_ident <#(#generics: Tokenizable + Parameterize),*> {
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
            let value = if let ParamType::Unit = field_type.param_type {
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
            if let ParamType::Unit = field_type.param_type {
                quote! { Self::#field_name() => (#u8_discriminant, ().into_token())}
            } else {
                quote! { Self::#field_name(inner) => (#u8_discriminant, inner.into_token())}
            }
        },
    );

    quote! {
            impl<#(#generics: Tokenizable + Parameterize),*> Tokenizable for #enum_ident <#(#generics),*> {
                fn from_token(token: Token) -> Result<Self, SDKError>
                where
                    Self: Sized,
                {
                    let gen_err = |msg| {
                        SDKError::InvalidData(format!(
                            "Error while instantiating {} from token! {}", #enum_ident_stringified, msg
                        ))
                    };
                    match token {
                        Token::Enum(selector) => {
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

                fn into_token(self) -> Token {
                    let (discriminant, token) = match self {
                        #(#match_discriminant_into_token),*
                    };

                    let variants = match Self::param_type() {
                        ParamType::Enum(variants) => variants,
                        other => panic!("Calling {}::param_type() must return a ParamType::Enum but instead it returned: {}", #enum_ident_stringified, other)
                    };

                    Token::Enum(Box::new((discriminant, token, variants)))
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
        impl<#(#generics: Parameterize + Tokenizable),*> Parameterize for #enum_ident <#(#generics),*> {
            fn param_type() -> ParamType {
                let mut param_types = vec![];
                #(param_types.push(#param_type_calls);)*

                let variants = EnumVariants::new(param_types).unwrap_or_else(|_| panic!("{} has no variants which isn't allowed!", #enum_ident_stringified));
                ParamType::Enum(variants)
            }
        }
    }
}
