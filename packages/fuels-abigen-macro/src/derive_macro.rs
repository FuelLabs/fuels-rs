use proc_macro2::TokenStream;
use quote::quote;
use quote::ToTokens;
use syn::{Data, DeriveInput};

pub fn generate_parameterize_impl(input: DeriveInput) -> syn::Result<TokenStream> {
    let struct_name = input.ident;

    let fields = match input.data {
        Data::Struct(struct_contents) => struct_contents.fields,
        _ => {
            panic!("Nije trebalo ovo metchat")
        }
    };

    let (impl_gen, type_gen, where_clause) = input.generics.split_for_impl();

    let struct_name_str = struct_name.to_string();
    let field_pairs = fields
        .into_iter()
        .map(|field| {
            let ident = field.ident.unwrap().to_string();
            let ttype = field.ty.to_token_stream();

            quote! {(#ident.to_string(), <#ttype as ::fuels::core::traits::Parameterize>::param_type())}
        })
        .collect::<Vec<_>>();

    let generic_param_types = input
        .generics
        .params
        .iter()
        .map(|generic_param| match generic_param {
            syn::GenericParam::Type(type_param) => {
                let ident = &type_param.ident;
                quote! {<#ident as ::fuels::core::traits::Parameterize>::param_type()}
            }
            _ => {
                panic!("Should only have types as generics")
            }
        })
        .collect::<Vec<_>>();

    Ok(quote! {
        impl #impl_gen ::fuels::core::traits::Parameterize for #struct_name #type_gen #where_clause {
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
