use proc_macro2::{Ident, TokenStream};
use quote::{quote, ToTokens};
use syn::{Data, DataEnum, DataStruct, DeriveInput, Error, Fields, Generics};

use parse_utils::extract_enum_members;

use crate::parse_utils;
use crate::parse_utils::{extract_generic_types, extract_struct_members, Members};

pub fn generate_parameterize_impl(input: DeriveInput) -> syn::Result<TokenStream> {
    match input.data {
        Data::Struct(struct_contents) => {
            parameterize_for_struct(input.ident, input.generics, struct_contents)
        }
        Data::Enum(enum_contents) => {
            parameterize_for_enum(input.ident, input.generics, enum_contents)
        }
        _ => Err(Error::new_spanned(input, "Union type is not supported")),
    }
}

fn parameterize_for_struct(
    name: Ident,
    generics: Generics,
    contents: DataStruct,
) -> Result<TokenStream, Error> {
    let (impl_gen, type_gen, where_clause) = generics.split_for_impl();

    let name_stringified = name.to_string();

    let members = extract_struct_members(contents.fields)?;

    let field_names = members.names_as_strings();
    let param_type_calls = members.param_type_calls();

    let generic_param_types = parameterize_generic_params(&generics)?;

    Ok(quote! {
        impl #impl_gen ::fuels::types::traits::Parameterize for #name #type_gen #where_clause {
            fn param_type() -> ::fuels::types::param_types::ParamType {
                ::fuels::types::param_types::ParamType::Struct{
                    name: #name_stringified.to_string(),
                    fields: vec![#((#field_names, #param_type_calls)),*],
                    generics: vec![#(#generic_param_types),*],
                }
            }
        }
    })
}

fn parameterize_generic_params(generics: &Generics) -> syn::Result<Vec<TokenStream>> {
    let parameterize_calls = extract_generic_types(generics)?
        .into_iter()
        .map(|type_param| {
            let ident = &type_param.ident;
            quote! {<#ident as ::fuels::types::traits::Parameterize>::param_type()}
        })
        .collect();

    Ok(parameterize_calls)
}

fn parameterize_for_enum(
    name: Ident,
    generics: Generics,
    contents: DataEnum,
) -> Result<TokenStream, Error> {
    let (impl_gen, type_gen, where_clause) = generics.split_for_impl();

    let enum_name_str = name.to_string();

    let declarations = extract_enum_members(contents.variants)?;

    let variant_names = declarations.names_as_strings();

    let variant_param_types = declarations.param_type_calls();

    let generic_param_types = parameterize_generic_params(&generics)?;

    Ok(quote! {
        impl #impl_gen ::fuels::types::traits::Parameterize for #name #type_gen #where_clause {
            fn param_type() -> ::fuels::types::param_types::ParamType {
                let variants = vec![#((#variant_names, #variant_param_types)),*];

                let variants = ::fuels::types::enum_variants::EnumVariants::new(variants).unwrap_or_else(|_| panic!("{} has no variants which isn't allowed!", #enum_name_str));
                ::fuels::types::param_types::ParamType::Enum {
                    name: #enum_name_str.to_string(),
                    variants,
                    generics: [#(#generic_param_types),*].to_vec()
                }
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use syn::parse::{Parse, ParseStream};
    use syn::Fields;

    use super::*;

    #[test]
    fn something() -> syn::Result<()> {
        let derive_input = syn::parse::Parser::parse2(
            |stream: ParseStream| DeriveInput::parse(stream),
            quote! {
                    enum SomeEnum {
                        a(),
                        b
                    }
            },
        )?;

        let inner = match derive_input.data {
            Data::Enum(inner) => inner,
            _ => {
                panic!("")
            }
        };

        inner
            .variants
            .into_iter()
            .for_each(|variant| match variant.fields {
                Fields::Named(named) => {
                    eprintln!("named");
                    named
                        .named
                        .iter()
                        .for_each(|f| eprintln!("{:?}: {}", f.ident, f.ty.to_token_stream()));
                }
                Fields::Unnamed(unnamed) => {
                    eprintln!("unnamed");
                    unnamed
                        .unnamed
                        .iter()
                        .for_each(|f| eprintln!("{:?}: {}", f.ident, f.ty.to_token_stream()));
                }
                Fields::Unit => {
                    eprintln!("unit")
                }
            });

        panic!("Stop right there");
    }
}
