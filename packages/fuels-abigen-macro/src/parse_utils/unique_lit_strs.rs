use crate::parse_utils;
use itertools::{chain, Itertools};
use parse_utils::{validate_no_duplicates, ErrorsExt};
use proc_macro2::Span;
use quote::ToTokens;
use std::vec::IntoIter;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{Error, Lit, LitStr, NestedMeta};

#[derive(Debug)]
pub struct UniqueLitStrs {
    span: Span,
    lit_strs: Vec<LitStr>,
}

impl UniqueLitStrs {
    pub fn new<T: ToTokens>(nested_metas: Punctuated<NestedMeta, T>) -> Result<Self, Error> {
        let span = nested_metas.span();

        let (lit_strs, errors): (Vec<_>, Vec<_>) = nested_metas
            .into_iter()
            .map(|meta| {
                if let NestedMeta::Lit(Lit::Str(lit_str)) = meta {
                    Ok(lit_str)
                } else {
                    Err(Error::new_spanned(meta, "Expected a string!"))
                }
            })
            .partition_result();

        let maybe_error = validate_no_duplicates(&lit_strs, |e| e.value()).err();

        chain!(errors, maybe_error).validate_no_errors()?;

        Ok(Self { span, lit_strs })
    }

    #[allow(dead_code)]
    pub fn iter(&self) -> impl Iterator<Item = &LitStr> {
        self.lit_strs.iter()
    }

    #[allow(dead_code)]
    pub fn span(&self) -> Span {
        self.span
    }
}

impl IntoIterator for UniqueLitStrs {
    type Item = LitStr;
    type IntoIter = IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.lit_strs.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_utils::Command;
    use proc_macro2::TokenStream;
    use quote::quote;

    #[test]
    fn correctly_reads_lit_strs() -> syn::Result<()> {
        // given
        let stream = quote! {SomeCommand("lit1", "lit2")};

        // when
        let unique_lit_strs = parse_unique_lit_strs(stream)?;

        // then
        let stringified = unique_lit_strs
            .iter()
            .map(|lit_str| lit_str.value())
            .collect::<Vec<_>>();

        assert_eq!(stringified, vec!["lit1", "lit2"]);

        Ok(())
    }

    #[test]
    fn doesnt_allow_duplicates() {
        // given
        let stream = quote! {SomeCommand("lit1", "lit2", "lit1")};

        // when
        let err = parse_unique_lit_strs(stream).expect_err("Should have failed");

        // then
        let messages = err.into_iter().map(|e| e.to_string()).collect::<Vec<_>>();
        assert_eq!(messages, vec!["Original defined here:", "Duplicate!"]);
    }

    #[test]
    fn only_strings_allowed() {
        let stream = quote! {SomeCommand("lit1", "lit2", true)};

        let err = parse_unique_lit_strs(stream).expect_err("Should have failed");

        assert_eq!(err.to_string(), "Expected a string!");
    }

    fn parse_unique_lit_strs(stream: TokenStream) -> syn::Result<UniqueLitStrs> {
        let command = Command::parse_single_from_token_stream(stream)?;

        UniqueLitStrs::new(command.contents)
    }
}
