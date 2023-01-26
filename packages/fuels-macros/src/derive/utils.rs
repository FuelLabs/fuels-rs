use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::{Attribute, Error};

use crate::{
    abigen::TypePath,
    parse_utils::{Command, UniqueLitStrs},
};

pub(crate) fn determine_fuels_types_path(attributes: &[Attribute]) -> syn::Result<TokenStream> {
    Ok(extract_fuels_types_path(attributes)?
        .map(|result| result.to_token_stream())
        .unwrap_or_else(|| quote! {::fuels::types}))
}

fn extract_fuels_types_path(attrs: &[Attribute]) -> syn::Result<Option<TypePath>> {
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
