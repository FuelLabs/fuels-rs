// pub(crate) fn impl_try_from(ident: &Ident, generics: &[TokenStream]) -> TokenStream {
// }

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Error};

use crate::{abigen_macro::TypePath, parameterize_macro::extract_fuels_types_path};

pub fn generate_try_from_impl(input: DeriveInput) -> syn::Result<TokenStream> {
    let fuels_types_path = extract_fuels_types_path(&input.attrs)?
        .unwrap_or_else(|| TypePath::new("::fuels::types").expect("Known to be correct"));

    match input.data {
        Data::Struct(_) => impl_try_from(input, fuels_types_path),
        Data::Enum(_) => impl_try_from(input, fuels_types_path),
        _ => {
            panic!("Union type is not supported")
        }
    }
}

fn impl_try_from(input: DeriveInput, fuels_types_path: TypePath) -> Result<TokenStream, Error> {
    let name = &input.ident;
    let (impl_gen, type_gen, where_clause) = input.generics.split_for_impl();

    Ok(quote! {

        impl #impl_gen TryFrom<&[u8]> for #name #type_gen #where_clause {
            type Error = #fuels_types_path::errors::Error;

            fn try_from(bytes: &[u8]) -> ::std::result::Result<Self, Self::Error> {
                ::fuels::core::try_from_bytes(bytes)
            }
        }

        impl #impl_gen TryFrom<&::std::vec::Vec<u8>> for #name #type_gen #where_clause {
            type Error = #fuels_types_path::errors::Error;

            fn try_from(bytes: &::std::vec::Vec<u8>) -> ::std::result::Result<Self, Self::Error> {
                ::fuels::core::try_from_bytes(&bytes)
            }
        }

        impl #impl_gen TryFrom<::std::vec::Vec<u8>> for #name #type_gen #where_clause {
            type Error = #fuels_types_path::errors::Error;

            fn try_from(bytes: ::std::vec::Vec<u8>) -> ::std::result::Result<Self, Self::Error> {
                ::fuels::core::try_from_bytes(&bytes)
            }
        }
    })
}
