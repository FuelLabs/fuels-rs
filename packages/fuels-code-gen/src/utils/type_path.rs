use std::str::FromStr;

use proc_macro2::{Ident, TokenStream};
use quote::{quote, ToTokens};

use crate::error::{error, Result};
use crate::utils::ident;

#[derive(Clone, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct TypePath {
    parts: Vec<Ident>,
    is_absolute: bool,
}

impl TypePath {
    pub fn new<T: ToString>(path: T) -> Result<Self> {
        let path_str = path.to_string();
        let is_absolute = Self::is_absolute(&path_str);

        let parts = path_str
            .split("::")
            .skip(is_absolute as usize)
            .map(|part| {
                let trimmed_part = part.trim().to_string();
                if trimmed_part.is_empty() {
                    return Err(error!("TypePath cannot be constructed from '{path_str}' since it has it has empty parts"))
                }
                Ok(ident(&trimmed_part))
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(Self { parts, is_absolute })
    }

    fn is_absolute(path_str: &str) -> bool {
        path_str.trim_start().starts_with("::")
    }

    pub fn prepend(self, mut another: TypePath) -> Self {
        another.parts.extend(self.parts);
        another
    }

    pub fn type_name(&self) -> String {
        self.ident().to_string()
    }

    pub fn ident(&self) -> &Ident {
        &self.parts.last().expect("Must have at least one element")
    }
}

impl ToTokens for TypePath {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let parts = &self.parts;
        let leading_delimiter = self.is_absolute.then_some(quote! {::});

        tokens.extend(quote! { #leading_delimiter #(#parts)::* });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cannot_be_empty() {
        let empty_path = "   ";

        let err = TypePath::new(empty_path).expect_err("Should have failed!");

        assert_eq!(
            err.to_string(),
            "TypePath cannot be constructed from '   ' since it has it has empty parts"
        );
    }

    #[test]
    fn must_have_ident_at_end() {
        let no_ident = "  ::missing_ident:: ";

        let err = TypePath::new(no_ident).expect_err("Should have failed!");

        assert_eq!(
            err.to_string(),
            "TypePath cannot be constructed from '  ::missing_ident:: ' since it has it has empty parts"
        );
    }

    #[test]
    fn trims_whitespace() {
        let path = " some_mod :: ident ";

        let path = TypePath::new(path).expect("Should have passed.");

        assert_eq!(path.parts, vec!["some_mod", "ident"])
    }

    #[test]
    fn can_be_prepended_to() {
        let path = TypePath::new(" some_mod :: ident ").expect("Should have passed.");
        let another_path = TypePath::new(" something :: else ").expect("the type path is valid");

        let joined = path.prepend(another_path);

        assert_eq!(joined.parts, vec!["something", "else", "some_mod", "ident"])
    }

    #[test]
    fn can_get_type_name() {
        let path = TypePath::new(" some_mod :: ident ").expect("Should have passed.");

        let type_name = path.type_name();

        assert_eq!(type_name, "ident");
    }

    #[test]
    fn can_handle_absolute_paths() {
        let absolute_path = " ::std :: vec:: Vec";

        let type_path = TypePath::new(absolute_path);

        type_path.unwrap();
    }

    #[test]
    fn leading_delimiter_present_when_path_is_absolute() {
        let type_path = TypePath::new(" ::std :: vec:: Vec").unwrap();

        let tokens = type_path.to_token_stream();

        let expected = quote! {::std::vec::Vec};
        assert_eq!(expected.to_string(), tokens.to_string())
    }

    #[test]
    fn leading_delimiter_not_present_when_path_is_relative() {
        let type_path = TypePath::new(" std :: vec:: Vec").unwrap();

        let tokens = type_path.to_token_stream();

        let expected = quote! {std::vec::Vec};
        assert_eq!(expected.to_string(), tokens.to_string())
    }
}
