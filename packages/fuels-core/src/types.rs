use crate::errors::Error;
use anyhow::Result;
use proc_macro2::TokenStream;
use quote::quote;

use crate::ParamType;

/// Expands a [`ParamType`] into a TokenStream.
/// Used to expand functions when generating type-safe bindings of a JSON ABI.
pub fn expand_type(kind: &ParamType) -> Result<TokenStream, Error> {
    match kind {
        ParamType::Unit => Ok(quote! {}),
        ParamType::U8 | ParamType::Byte => Ok(quote! { u8 }),
        ParamType::U16 => Ok(quote! { u16 }),
        ParamType::U32 => Ok(quote! { u32 }),
        ParamType::U64 => Ok(quote! { u64 }),
        ParamType::Bool => Ok(quote! { bool }),
        ParamType::B256 => Ok(quote! { [u8; 32] }),
        ParamType::String(_) => Ok(quote! { String }),
        ParamType::Array(t, _size) => {
            let inner = expand_type(t)?;
            Ok(quote! { ::std::vec::Vec<#inner> })
        }
        ParamType::Struct(members) => {
            if members.is_empty() {
                return Err(Error::InvalidData("struct members can not be empty".into()));
            }
            let members = members
                .iter()
                .map(expand_type)
                .collect::<Result<Vec<_>, _>>()?;
            Ok(quote! { (#(#members,)*) })
        }
        ParamType::Enum(members) => {
            if members.is_empty() {
                return Err(Error::InvalidData("enum members can not be empty".into()));
            }
            let members = members
                .iter()
                .map(expand_type)
                .collect::<Result<Vec<_>, _>>()?;
            Ok(quote! { (#(#members,)*) })
        }
        ParamType::Tuple(members) => {
            if members.is_empty() {
                return Err(Error::InvalidData("tuple members can not be empty".into()));
            }

            let members = members
                .iter()
                .map(expand_type)
                .collect::<Result<Vec<_>, _>>()?;
            Ok(quote! { (#(#members,)*) })
        }
    }
}
