use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Error, Result};

use crate::derive::utils::get_path_from_attr_or;

pub fn generate_try_from_impl(input: DeriveInput) -> Result<TokenStream> {
    let fuels_types_path =
        get_path_from_attr_or("FuelsTypesPath", &input.attrs, quote! {::fuels::types})?;
    let fuels_core_path =
        get_path_from_attr_or("FuelsCorePath", &input.attrs, quote! {::fuels::core})?;

    match input.data {
        Data::Enum(_) | Data::Struct(_) => impl_try_from(input, fuels_types_path, fuels_core_path),
        Data::Union(union) => Err(Error::new_spanned(
            union.union_token,
            "Unions are not supported.",
        )),
    }
}

fn impl_try_from(
    input: DeriveInput,
    fuels_types_path: TokenStream,
    fuels_core_path: TokenStream,
) -> Result<TokenStream> {
    let name = &input.ident;
    let (impl_gen, type_gen, where_clause) = input.generics.split_for_impl();

    Ok(quote! {
        impl #impl_gen TryFrom<&[u8]> for #name #type_gen #where_clause {
            type Error = #fuels_types_path::errors::Error;

            fn try_from(bytes: &[u8]) -> #fuels_types_path::errors::Result<Self> {
                #fuels_core_path::try_from_bytes(bytes)
            }
        }

        impl #impl_gen TryFrom<&::std::vec::Vec<u8>> for #name #type_gen #where_clause {
            type Error = #fuels_types_path::errors::Error;

            fn try_from(bytes: &::std::vec::Vec<u8>) -> #fuels_types_path::errors::Result<Self> {
                #fuels_core_path::try_from_bytes(&bytes)
            }
        }

        impl #impl_gen TryFrom<::std::vec::Vec<u8>> for #name #type_gen #where_clause {
            type Error = #fuels_types_path::errors::Error;

            fn try_from(bytes: ::std::vec::Vec<u8>) -> #fuels_types_path::errors::Result<Self> {
                #fuels_core_path::try_from_bytes(&bytes)
            }
        }
    })
}
