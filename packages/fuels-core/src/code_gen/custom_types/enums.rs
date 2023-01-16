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
/// `Tokenizable` and `TryFrom` implementations for the enum described by the
/// given TypeDeclaration.
pub(crate) fn expand_custom_enum(
    type_decl: &FullTypeDeclaration,
    shared_types: &HashSet<FullTypeDeclaration>,
) -> Result<GeneratedCode, Error> {
    let enum_name = custom_type_name(&type_decl.type_field)?;
    let enum_ident = ident(&enum_name);

    let components = extract_components(type_decl, false, shared_types)?;
    if components.is_empty() {
        return Err(Error::InvalidData(
            "Enum must have at least one component!".into(),
        ));
    }
    let generics = extract_generic_parameters(type_decl)?;

    let enum_def = enum_decl(&enum_ident, &components, &generics);
    let parameterize_impl = enum_parameterize_impl(&enum_ident, &components, &generics);
    let tokenize_impl = enum_tokenizable_impl(&enum_ident, &components, &generics);
    let try_from = impl_try_from(&enum_ident, &generics);

    let code = quote! {
        #enum_def

        #parameterize_impl

        #tokenize_impl

        #try_from
    };
    Ok(GeneratedCode {
        code,
        usable_types: HashSet::from([TypePath::new(&enum_name).expect("Enum name is not empty!")]),
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
        #[allow(clippy::enum_variant_names)]
        #[derive(Clone, Debug, Eq, PartialEq)]
        pub enum #enum_ident <#(#generics: ::fuels::core::traits::Tokenizable + ::fuels::core::traits::Parameterize),*> {
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
                quote! { ::fuels::core::traits::Tokenizable::from_token(variant_token)? }
            };

            let u8_discriminant = discriminant as u8;
            quote! { #u8_discriminant => ::std::result::Result::Ok(Self::#field_name(#value))}
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
                quote! { Self::#field_name(inner) => (#u8_discriminant, ::fuels::core::traits::Tokenizable::into_token(inner))}
            }
        },
    );

    quote! {
            impl<#(#generics: ::fuels::core::traits::Tokenizable + ::fuels::core::traits::Parameterize),*> ::fuels::core::traits::Tokenizable for self::#enum_ident <#(#generics),*> {
                fn from_token(token: ::fuels::types::Token) -> ::std::result::Result<Self, ::fuels::types::errors::Error>
                where
                    Self: Sized,
                {
                    let gen_err = |msg| {
                        ::fuels::types::errors::Error::InvalidData(format!(
                            "Error while instantiating {} from token! {}", #enum_ident_stringified, msg
                        ))
                    };
                    match token {
                        ::fuels::types::Token::Enum(selector) => {
                            let (discriminant, variant_token, _) = *selector;
                            match discriminant {
                                #(#match_discriminant_from_token,)*
                                _ => ::std::result::Result::Err(gen_err(format!(
                                    "Discriminant {} doesn't point to any of the enums variants.", discriminant
                                ))),
                            }
                        }
                        _ => ::std::result::Result::Err(gen_err(format!(
                            "Given token ({}) is not of the type Token::Enum!", token
                        ))),
                    }
                }

                fn into_token(self) -> ::fuels::types::Token {
                    let (discriminant, token) = match self {
                        #(#match_discriminant_into_token),*
                    };

                    let variants = match <Self as ::fuels::core::traits::Parameterize>::param_type() {
                        ::fuels::types::param_types::ParamType::Enum{variants, ..} => variants,
                        other => panic!("Calling {}::param_type() must return a ParamType::Enum but instead it returned: {:?}", #enum_ident_stringified, other)
                    };

                    ::fuels::types::Token::Enum(::std::boxed::Box::new((discriminant, token, variants)))
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
    let variants = components
        .iter()
        .map(|component| {
            let type_name = component.field_name.to_string();
            quote! {#type_name.to_string()}
        })
        .zip(param_type_calls)
        .map(|(type_name, param_type_call)| {
            quote! {(#type_name, #param_type_call)}
        });
    let enum_ident_stringified = enum_ident.to_string();
    quote! {
        impl<#(#generics: ::fuels::core::traits::Parameterize + ::fuels::core::traits::Tokenizable),*> ::fuels::core::traits::Parameterize for self::#enum_ident <#(#generics),*> {
            fn param_type() -> ::fuels::types::param_types::ParamType {
                let variants = [#(#variants),*].to_vec();

                let variants = ::fuels::types::enum_variants::EnumVariants::new(variants).unwrap_or_else(|_| panic!("{} has no variants which isn't allowed!", #enum_ident_stringified));
                ::fuels::types::param_types::ParamType::Enum{
                    name: #enum_ident_stringified.to_string(),
                    variants,
                    generics: [#(#generics::param_type()),*].to_vec()
                }
            }
        }
    }
}
