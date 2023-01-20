use proc_macro2::TokenStream;
use quote::quote;
use quote::ToTokens;
use syn::{Data, DataEnum, DataStruct, DeriveInput, Error};

pub fn generate_parameterize_impl(input: DeriveInput) -> syn::Result<TokenStream> {
    match &input.data {
        Data::Struct(struct_contents) => parameterize_struct(&input, struct_contents),
        Data::Enum(enum_contents) => parameterize_enum(&input, enum_contents),
        _ => {
            panic!("Union type is not supported")
        }
    }
}

fn parameterize_struct(
    input: &DeriveInput,
    struct_contents: &DataStruct,
) -> Result<TokenStream, Error> {
    let struct_name = &input.ident;

    let (impl_gen, type_gen, where_clause) = input.generics.split_for_impl();

    let struct_name_str = struct_name.to_string();
    let field_pairs = &struct_contents.fields
        .iter()
        .map(|field| {
            let ident = field.ident.as_ref().unwrap().to_string();
            let ttype = field.ty.to_token_stream();

            quote! {(#ident.to_string(), <#ttype as ::fuels::types::traits::Parameterize>::param_type())}
        })
        .collect::<Vec<_>>();

    let generic_param_types = input
        .generics
        .params
        .iter()
        .map(|generic_param| match generic_param {
            syn::GenericParam::Type(type_param) => {
                let ident = &type_param.ident;
                quote! {<#ident as ::fuels::types::traits::Parameterize>::param_type()}
            }
            _ => {
                panic!("Should only have types as generics")
            }
        })
        .collect::<Vec<_>>();

    Ok(quote! {
        impl #impl_gen ::fuels::types::traits::Parameterize for #struct_name #type_gen #where_clause {
            fn param_type() -> ::fuels::types::param_types::ParamType {
                ::fuels::types::param_types::ParamType::Struct{
                    name: #struct_name_str.to_string(),
                    fields: vec![#(#field_pairs),*],
                    generics: vec![#(#generic_param_types),*],
                }
            }
        }
    })
}

fn parameterize_enum(
    _input: &DeriveInput,
    _enum_contents: &DataEnum,
) -> Result<TokenStream, Error> {
    todo!()
}
