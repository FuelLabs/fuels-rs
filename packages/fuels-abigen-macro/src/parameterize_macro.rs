use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::{
    Data, DataEnum, DataStruct, DeriveInput, Error,
    Fields::{Unit, Unnamed},
};

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

fn parameterize_enum(input: &DeriveInput, enum_contents: &DataEnum) -> Result<TokenStream, Error> {
    let enum_name = &input.ident;

    let (impl_gen, type_gen, where_clause) = input.generics.split_for_impl();

    let enum_name_str = enum_name.to_string();

    let variants = enum_contents.variants.iter().map(|v| {
        let name = v.ident.to_string();

        let field = match &v.fields {
            Unnamed(fields_unnamed) => fields_unnamed
                .unnamed
                .iter()
                .next()
                .map(|f| f.ty.clone().into_token_stream())
                .unwrap_or(quote! {()}),
            Unit => quote! {()},
            //TODO: make nice syn error from this
            _ => panic!("Named variants not supported"),
        };

        quote!{ (#name.to_string(), <#field as ::fuels::types::traits::Parameterize>::param_type())}
    });

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
        impl #impl_gen ::fuels::types::traits::Parameterize for #enum_name #type_gen #where_clause {
            fn param_type() -> ::fuels::types::param_types::ParamType {
                let variants = [#(#variants),*].to_vec();

                let variants = ::fuels::types::enum_variants::EnumVariants::new(variants).unwrap_or_else(|_| panic!("{} has no variants which isn't allowed!", #enum_name_str));
                ::fuels::types::param_types::ParamType::Enum{
                    name: #enum_name_str.to_string(),
                    variants,
                    generics: [#(#generic_param_types),*].to_vec()
                }
            }
        }
    })
}
