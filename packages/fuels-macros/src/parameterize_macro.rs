use proc_macro2::{Ident, TokenStream};
use quote::{quote, ToTokens};
use syn::{Attribute, Data, DataEnum, DataStruct, DeriveInput, Error, Generics, Path};

use crate::{
    abigen_macro::TypePath,
    parse_utils::{
        extract_enum_members, extract_generic_types, extract_struct_members, Command, UniqueLitStrs,
    },
};

pub(crate) fn extract_fuels_types_path(attrs: &[Attribute]) -> syn::Result<Option<TypePath>> {
    let maybe_command = attrs
        .iter()
        .find(|attr| {
            attr.path
                .get_ident()
                .map(|ident| ident == "FuelsTypesPath")
                .unwrap_or(false)
        })
        .map(|attr| attr.tokens.clone());

    if maybe_command.is_none() {
        return Ok(None);
    }
    let tokens = maybe_command.unwrap();

    let code = quote! {FuelsTypesPath #tokens};

    let command = Command::parse_single_from_token_stream(code)?;

    let unique_lit_strs = UniqueLitStrs::new(command.contents)?;
    let contents_span = unique_lit_strs.span();

    match unique_lit_strs.into_iter().collect::<Vec<_>>().as_slice() {
        [single_item] => {
            let type_path = TypePath::new(single_item.value())
                .map_err(|_| Error::new_spanned(single_item.clone(), "Invalid Path"))?;

            Ok(Some(type_path))
        }
        _ => Err(Error::new(contents_span, "Must contain exactly one Path!")),
    }
}

pub fn generate_parameterize_impl(input: DeriveInput) -> syn::Result<TokenStream> {
    let fuels_types_path = extract_fuels_types_path(&input.attrs)?
        .unwrap_or_else(|| TypePath::new("::fuels::types").expect("Known to be correct"));

    match input.data {
        Data::Struct(struct_contents) => parameterize_for_struct(
            input.ident,
            input.generics,
            struct_contents,
            fuels_types_path,
        ),
        Data::Enum(enum_contents) => {
            parameterize_for_enum(input.ident, input.generics, enum_contents, fuels_types_path)
        }
        _ => Err(Error::new_spanned(input, "Union type is not supported")),
    }
}

fn parameterize_for_struct(
    name: Ident,
    generics: Generics,
    contents: DataStruct,
    fuels_types_path: TypePath,
) -> Result<TokenStream, Error> {
    let (impl_gen, type_gen, where_clause) = generics.split_for_impl();
    let name_stringified = name.to_string();
    let members = extract_struct_members(contents, fuels_types_path.clone())?;
    let field_names = members.names_as_strings();
    let param_type_calls = members.param_type_calls();
    let generic_param_types = parameterize_generic_params(&generics, fuels_types_path.clone())?;

    Ok(quote! {
        impl #impl_gen #fuels_types_path::traits::Parameterize for #name #type_gen #where_clause {
            fn param_type() -> #fuels_types_path::param_types::ParamType {
                #fuels_types_path::param_types::ParamType::Struct{
                    name: #name_stringified.to_string(),
                    fields: vec![#((#field_names, #param_type_calls)),*],
                    generics: vec![#(#generic_param_types),*],
                }
            }
        }
    })
}

fn parameterize_generic_params(
    generics: &Generics,
    fuels_types_path: TypePath,
) -> syn::Result<Vec<TokenStream>> {
    let parameterize_calls = extract_generic_types(generics)?
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
    fuels_types_path: TypePath,
) -> Result<TokenStream, Error> {
    let (impl_gen, type_gen, where_clause) = generics.split_for_impl();
    let enum_name_str = name.to_string();
    let declarations = extract_enum_members(contents, fuels_types_path.clone())?;
    let variant_names = declarations.names_as_strings();
    let variant_param_types = declarations.param_type_calls();
    let generic_param_types = parameterize_generic_params(&generics, fuels_types_path.clone())?;

    Ok(quote! {
        impl #impl_gen #fuels_types_path::traits::Parameterize for #name #type_gen #where_clause {
            fn param_type() -> #fuels_types_path::param_types::ParamType {
                let variants = vec![#((#variant_names, #variant_param_types)),*];

                let variants = #fuels_types_path::enum_variants::EnumVariants::new(variants).unwrap_or_else(|_| panic!("{} has no variants which isn't allowed!", #enum_name_str));
                #fuels_types_path::param_types::ParamType::Enum {
                    name: #enum_name_str.to_string(),
                    variants,
                    generics: [#(#generic_param_types),*].to_vec()
                }
            }
        }
    })
}
