use std::str::FromStr;

use proc_macro2::TokenStream;
use quote::{quote, ToTokens};

use crate::error::{error, Result};

#[derive(Clone, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) struct TypePath {
    parts: Vec<String>,
}

impl TypePath {
    pub fn new<T: ToString>(path: T) -> Result<Self> {
        let path_str = path.to_string().trim().to_string();

        if path_str.is_empty() {
            return Err(error!(
                "TypePath cannot be constructed from '{path_str}' because it's empty!"
            ));
        }

        let parts = path_str
            .split("::")
            .map(|part| part.trim().to_string())
            .collect::<Vec<_>>();

        let type_name = parts
            .last()
            .expect("Cannot be empty, since we started off with a non-empty string");

        if type_name.is_empty() {
            Err(error!(
                "TypePath cannot be constructed from '{path_str}'! Missing ident at the end."
            ))
        } else {
            Ok(Self { parts })
        }
    }

    pub fn prepend(self, mut another: TypePath) -> Self {
        another.parts.extend(self.parts);
        another
    }

    pub fn type_name(&self) -> &str {
        self.parts
            .last()
            .expect("Must have at least one element")
            .as_str()
    }
}

impl ToTokens for TypePath {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let parts = self
            .parts
            .iter()
            .map(|part| TokenStream::from_str(part).unwrap());

        let tokenized_parts = quote! { #(#parts)::* };

        tokens.extend(tokenized_parts);
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
            "TypePath cannot be constructed from '' because it's empty!"
        );
    }

    #[test]
    fn must_have_ident_at_end() {
        let no_ident = "  ::missing_ident:: ";

        let err = TypePath::new(no_ident).expect_err("Should have failed!");

        assert_eq!(
            err.to_string(),
            "TypePath cannot be constructed from '::missing_ident::'! Missing ident at the end."
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
}
