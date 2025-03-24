use std::vec::IntoIter;

use proc_macro2::{Span, TokenStream};
use syn::{LitStr, Result, parse::Parser, punctuated::Punctuated, spanned::Spanned, token::Comma};

use crate::parse_utils::validate_no_duplicates;

#[derive(Debug)]
pub struct UniqueLitStrs {
    span: Span,
    lit_strs: Vec<LitStr>,
}

impl UniqueLitStrs {
    pub fn new(tokens: TokenStream) -> Result<Self> {
        let parsed_lit_strs = Punctuated::<LitStr, Comma>::parse_terminated.parse2(tokens)?;
        let span = parsed_lit_strs.span();
        let lit_strs: Vec<_> = parsed_lit_strs.into_iter().collect();

        validate_no_duplicates(&lit_strs, |ls| ls.value())?;

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
    use proc_macro2::TokenStream;
    use quote::quote;

    use super::*;
    use crate::parse_utils::Command;

    #[test]
    fn correctly_reads_lit_strs() -> Result<()> {
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
        let err = parse_unique_lit_strs(stream).expect_err("should have failed");

        // then
        let messages = err.into_iter().map(|e| e.to_string()).collect::<Vec<_>>();
        assert_eq!(messages, vec!["original defined here:", "duplicate!"]);
    }

    #[test]
    fn only_strings_allowed() {
        let stream = quote! {SomeCommand("lit1", "lit2", true)};

        let err = parse_unique_lit_strs(stream).expect_err("should have failed");

        assert_eq!(err.to_string(), "expected string literal");
    }

    fn parse_unique_lit_strs(stream: TokenStream) -> Result<UniqueLitStrs> {
        let command = Command::parse_single_from_token_stream(stream)?;

        UniqueLitStrs::new(command.contents)
    }
}
