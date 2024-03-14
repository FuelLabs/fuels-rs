use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::{Data, DataEnum, DataStruct, DeriveInput, Error, Generics, Result};

use crate::{
    derive::utils::{find_attr, get_path_from_attr_or, std_lib_path},
    parse_utils::{validate_and_extract_generic_types, Members},
};

pub fn generate_parameterize_impl(input: DeriveInput) -> Result<TokenStream> {
    let fuels_types_path =
        get_path_from_attr_or("FuelsTypesPath", &input.attrs, quote! {::fuels::types})?;
    let fuels_core_path =
        get_path_from_attr_or("FuelsCorePath", &input.attrs, quote! {::fuels::core})?;
    let no_std = find_attr("NoStd", &input.attrs).is_some();

    match input.data {
        Data::Struct(struct_contents) => parameterize_for_struct(
            input.ident,
            input.generics,
            struct_contents,
            fuels_types_path,
            fuels_core_path,
            no_std,
        ),
        Data::Enum(enum_contents) => parameterize_for_enum(
            input.ident,
            input.generics,
            enum_contents,
            fuels_types_path,
            fuels_core_path,
            no_std,
        ),
        _ => Err(Error::new_spanned(input, "union type is not supported")),
    }
}

fn parameterize_for_struct(
    name: Ident,
    generics: Generics,
    contents: DataStruct,
    fuels_types_path: TokenStream,
    fuels_core_path: TokenStream,
    no_std: bool,
) -> Result<TokenStream> {
    let (impl_gen, type_gen, where_clause) = generics.split_for_impl();
    let name_stringified = name.to_string();
    let members = Members::from_struct(contents, fuels_core_path.clone())?;
    let field_names = members.names_as_strings();
    let param_type_calls = members.param_type_calls();
    let generic_param_types = parameterize_generic_params(&generics, &fuels_core_path)?;

    let std_lib = std_lib_path(no_std);

    Ok(quote! {
        impl #impl_gen #fuels_core_path::traits::Parameterize for #name #type_gen #where_clause {
            fn param_type() -> #fuels_types_path::param_types::ParamType {
                #fuels_types_path::param_types::ParamType::Struct{
                    name: #std_lib::string::String::from(#name_stringified),
                    fields: #std_lib::vec![#((#field_names, #param_type_calls)),*],
                    generics: #std_lib::vec![#(#generic_param_types),*],
                }
            }
        }
    })
}

fn parameterize_generic_params(
    generics: &Generics,
    fuels_core_path: &TokenStream,
) -> Result<Vec<TokenStream>> {
    let parameterize_calls = validate_and_extract_generic_types(generics)?
        .into_iter()
        .map(|type_param| {
            let ident = &type_param.ident;
            quote! {<#ident as #fuels_core_path::traits::Parameterize>::param_type()}
        })
        .collect();

    Ok(parameterize_calls)
}

fn parameterize_for_enum(
    name: Ident,
    generics: Generics,
    contents: DataEnum,
    fuels_types_path: TokenStream,
    fuels_core_path: TokenStream,
    no_std: bool,
) -> Result<TokenStream> {
    let (impl_gen, type_gen, where_clause) = generics.split_for_impl();
    let enum_name_str = name.to_string();
    let members = Members::from_enum(contents, fuels_core_path.clone())?;

    let variant_names = members.names_as_strings();
    let variant_param_types = members.param_type_calls();
    let generic_param_types = parameterize_generic_params(&generics, &fuels_core_path)?;

    let std_lib = std_lib_path(no_std);

    Ok(quote! {
        impl #impl_gen #fuels_core_path::traits::Parameterize for #name #type_gen #where_clause {
            fn param_type() -> #fuels_types_path::param_types::ParamType {
                let variants = #std_lib::vec![#((#variant_names, #variant_param_types)),*];
                let enum_variants = #fuels_types_path::param_types::EnumVariants::new(variants)
                    .unwrap_or_else(|_| ::std::panic!(
                            "{} has no variants which isn't allowed",
                            #enum_name_str
                        )
                    );

                #fuels_types_path::param_types::ParamType::Enum {
                    name: #std_lib::string::String::from(#enum_name_str),
                    enum_variants,
                    generics: #std_lib::vec![#(#generic_param_types),*]
                }
            }
        }
    })
}
