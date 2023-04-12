use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::{Data, DataEnum, DataStruct, DeriveInput, Error, Generics, Result};

use crate::{
    derive::utils::{find_attr, get_path_from_attr_or, std_lib_path},
    parse_utils::{
        extract_enum_members, extract_struct_members, validate_and_extract_generic_types,
    },
};

pub fn generate_parameterize_impl(input: DeriveInput) -> Result<TokenStream> {
    let fuels_types_path =
        get_path_from_attr_or("FuelsTypesPath", &input.attrs, quote! {::fuels::types})?;

    let no_std = find_attr("NoStd", &input.attrs).is_some();

    match input.data {
        Data::Struct(struct_contents) => parameterize_for_struct(
            input.ident,
            input.generics,
            struct_contents,
            fuels_types_path,
            no_std,
        ),
        Data::Enum(enum_contents) => parameterize_for_enum(
            input.ident,
            input.generics,
            enum_contents,
            fuels_types_path,
            no_std,
        ),
        _ => Err(Error::new_spanned(input, "Union type is not supported")),
    }
}

fn parameterize_for_struct(
    name: Ident,
    generics: Generics,
    contents: DataStruct,
    fuels_types_path: TokenStream,
    no_std: bool,
) -> Result<TokenStream> {
    let (impl_gen, type_gen, where_clause) = generics.split_for_impl();
    let members = extract_struct_members(contents, fuels_types_path.clone())?;
    let param_type_calls = members.param_type_calls();
    let generic_param_types = parameterize_generic_params(&generics, &fuels_types_path)?;

    let std_lib = std_lib_path(no_std);

    Ok(quote! {
        impl #impl_gen #fuels_types_path::traits::Parameterize for #name #type_gen #where_clause {
            fn param_type() -> #fuels_types_path::param_types::ParamType {
                #fuels_types_path::param_types::ParamType::Struct{
                    fields: #std_lib::vec![#(#param_type_calls),*],
                    generics: #std_lib::vec![#(#generic_param_types),*],
                }
            }
        }
    })
}

fn parameterize_generic_params(
    generics: &Generics,
    fuels_types_path: &TokenStream,
) -> Result<Vec<TokenStream>> {
    let parameterize_calls = validate_and_extract_generic_types(generics)?
        .into_iter()
        .map(|type_param| {
            let ident = &type_param.ident;
            quote! {<#ident as #fuels_types_path::traits::Parameterize>::param_type()}
        })
        .collect();

    Ok(parameterize_calls)
}

fn parameterize_for_enum(
    name: Ident,
    generics: Generics,
    contents: DataEnum,
    fuels_types_path: TokenStream,
    no_std: bool,
) -> Result<TokenStream> {
    let (impl_gen, type_gen, where_clause) = generics.split_for_impl();
    let enum_name_str = name.to_string();
    let members = extract_enum_members(contents, fuels_types_path.clone())?;

    let variant_param_types = members.param_type_calls();
    let generic_param_types = parameterize_generic_params(&generics, &fuels_types_path)?;

    let std_lib = std_lib_path(no_std);

    Ok(quote! {
        impl #impl_gen #fuels_types_path::traits::Parameterize for #name #type_gen #where_clause {
            fn param_type() -> #fuels_types_path::param_types::ParamType {
                let variants = #std_lib::vec![#(#variant_param_types),*];

                let variants = #fuels_types_path::enum_variants::EnumVariants::new(variants).unwrap_or_else(|_| ::std::panic!("{} has no variants which isn't allowed.", #enum_name_str));
                #fuels_types_path::param_types::ParamType::Enum {
                    variants,
                    generics: #std_lib::vec![#(#generic_param_types),*]
                }
            }
        }
    })
}
