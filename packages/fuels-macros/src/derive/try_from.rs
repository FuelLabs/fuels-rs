use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Error, Result};

use crate::derive::utils::{find_attr, get_path_from_attr_or, std_lib_path};

pub fn generate_try_from_impl(input: DeriveInput) -> Result<TokenStream> {
    let fuels_types_path =
        get_path_from_attr_or("FuelsTypesPath", &input.attrs, quote! {::fuels::types})?;
    let fuels_core_path =
        get_path_from_attr_or("FuelsCorePath", &input.attrs, quote! {::fuels::core})?;
    let no_std = find_attr("NoStd", &input.attrs).is_some();

    match input.data {
        Data::Enum(_) | Data::Struct(_) => {
            impl_try_from(input, fuels_types_path, fuels_core_path, no_std)
        }
        Data::Union(union) => Err(Error::new_spanned(
            union.union_token,
            "unions are not supported",
        )),
    }
}

fn impl_try_from(
    input: DeriveInput,
    fuels_types_path: TokenStream,
    fuels_core_path: TokenStream,
    no_std: bool,
) -> Result<TokenStream> {
    let name = &input.ident;
    let (impl_gen, type_gen, where_clause) = input.generics.split_for_impl();

    let std_lib = std_lib_path(no_std);
    Ok(quote! {
        impl #impl_gen TryFrom<&[u8]> for #name #type_gen #where_clause {
            type Error = #fuels_types_path::errors::Error;

            fn try_from(bytes: &[u8]) -> #fuels_types_path::errors::Result<Self> {
                #fuels_core_path::codec::try_from_bytes(bytes, ::std::default::Default::default())
            }
        }

        impl #impl_gen TryFrom<&#std_lib::vec::Vec<u8>> for #name #type_gen #where_clause {
            type Error = #fuels_types_path::errors::Error;

            fn try_from(bytes: &#std_lib::vec::Vec<u8>) -> #fuels_types_path::errors::Result<Self> {
                ::core::convert::TryInto::try_into(bytes.as_slice())
            }
        }

        impl #impl_gen TryFrom<#std_lib::vec::Vec<u8>> for #name #type_gen #where_clause {
            type Error = #fuels_types_path::errors::Error;

            fn try_from(bytes: #std_lib::vec::Vec<u8>) -> #fuels_types_path::errors::Result<Self> {
                ::core::convert::TryInto::try_into(bytes.as_slice())
            }
        }
    })
}
