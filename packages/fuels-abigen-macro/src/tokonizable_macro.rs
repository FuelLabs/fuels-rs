use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::{Data, DataEnum, DataStruct, DeriveInput, Error, Fields, Generics};

use parse_utils::extract_struct_members;

use crate::parse_utils;

pub fn generate_tokenizable_impl(input: DeriveInput) -> syn::Result<TokenStream> {
    match input.data {
        Data::Struct(struct_contents) => {
            tokenizable_for_struct(input.ident, input.generics, struct_contents)
        }
        Data::Enum(enum_contents) => {
            tokenizable_for_enum(input.ident, input.generics, enum_contents)
        }
        _ => Err(Error::new_spanned(input, "Union type is not supported")),
    }
}

fn tokenizable_for_struct(
    name: Ident,
    generics: Generics,
    contents: DataStruct,
) -> Result<TokenStream, Error> {
    let (impl_gen, type_gen, where_clause) = generics.split_for_impl();

    let struct_name_str = name.to_string();

    let field_names = extract_struct_members(contents.fields)?
        .names()
        .collect::<Vec<_>>();

    Ok(quote! {
        impl #impl_gen ::fuels::types::traits::Tokenizable for #name #type_gen #where_clause {
            fn into_token(self) -> ::fuels::types::Token {
                let tokens = [#(::fuels::types::traits::Tokenizable::into_token(self.#field_names)),*].to_vec();
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
                        ::std::result::Result::Ok(Self {
                            #(
                                #field_names: ::fuels::types::traits::Tokenizable::from_token(next_token()?)?
                             ),*

                        })
                    },
                    other => ::std::result::Result::Err(::fuels::types::errors::Error::InstantiationError(format!("Error while constructing '{}'. Expected token of type Token::Struct, got {:?}", #struct_name_str, other))),
                }
            }
        }
    })
}

fn tokenizable_for_enum(
    enum_name: Ident,
    generics: Generics,
    enum_contents: DataEnum,
) -> Result<TokenStream, Error> {
    let (impl_gen, type_gen, where_clause) = generics.split_for_impl();

    let enum_name_str = enum_name.to_string();

    let variants_fields: Vec<_> = enum_contents
        .variants
        .iter()
        .enumerate()
        .map(|(discriminant, variant)| {
            let name = variant.ident.clone();

            //TODO: use something else then 0,1,2
            let field = match &variant.fields {
                Fields::Unnamed(fields_unnamed) => {
                    fields_unnamed.unnamed.iter().next().map(|_| 0).unwrap_or(1)
                }
                Fields::Unit => 2,
                //TODO: make nice syn error from this
                _ => panic!("Named variants not supported"),
            };

            (name, discriminant as u8, field)
        })
        .collect();

    let discriminant_into_token = variants_fields.iter().map(|(name, discriminant, field)|{
        //TODO: use something else then 0,1,2
            if *field == 0 {
                quote! { Self::#name(inner) => (#discriminant, ::fuels::types::traits::Tokenizable::into_token(inner))}
            } else  if *field == 1 {
                quote! { Self::#name() => (#discriminant, ().into_token())}
            }
            else{
                quote! { Self::#name => (#discriminant, ().into_token())}
            }
    });

    let discriminant_from_token = variants_fields.iter().map(|(name, discriminant, field)| {
        //TODO: use something else then 0,1,2
        let self_name = if *field == 0 {
            quote! { #name(::fuels::types::traits::Tokenizable::from_token(variant_token)?) }
        } else if *field == 1 {
            quote! { #name() }
        } else {
            quote! { #name }
        };

        quote! { #discriminant => ::std::result::Result::Ok(Self::#self_name)}
    });

    Ok(quote! {
        impl #impl_gen ::fuels::types::traits::Tokenizable for #enum_name #type_gen #where_clause {
            fn into_token(self) -> ::fuels::types::Token {
                let (discriminant, token) = match self {
                    #(#discriminant_into_token),*
                };

                let variants = match <Self as ::fuels::types::traits::Parameterize>::param_type() {
                    ::fuels::types::param_types::ParamType::Enum{variants, ..} => variants,
                    other => panic!("Calling {}::param_type() must return a ParamType::Enum but instead it returned: {:?}", #enum_name_str, other)
                };

                ::fuels::types::Token::Enum(::std::boxed::Box::new((discriminant, token, variants)))
            }

            fn from_token(token: ::fuels::types::Token) -> ::std::result::Result<Self, ::fuels::types::errors::Error>
            where
                Self: Sized,
            {
                let gen_err = |msg| {
                    ::fuels::types::errors::Error::InvalidData(format!(
                        "Error while instantiating {} from token! {}", #enum_name_str, msg
                    ))
                };
                match token {
                    ::fuels::types::Token::Enum(selector) => {
                        let (discriminant, variant_token, _) = *selector;
                        match discriminant {
                            #(#discriminant_from_token,)*
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
        }
    })
}

enum SomeEnum {
    a(),
    b,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sa() {
        match SomeEnum::b {
            SomeEnum::a() => {}
            SomeEnum::b => {}
        }
    }
}
