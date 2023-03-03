use std::cmp::min;

use itertools::{chain, izip, Itertools};
use proc_macro2::{Ident, TokenStream};
use quote::{quote, ToTokens};

use crate::error::Error;
use crate::{
    error::{error, Result},
    utils::ident,
};

#[derive(Clone, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct TypePath {
    parts: Vec<Ident>,
    is_absolute: bool,
}

impl From<&Ident> for TypePath {
    fn from(value: &Ident) -> Self {
        TypePath::new(value).expect("All Idents are valid TypePaths")
    }
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

    fn len(&self) -> usize {
        self.parts.len()
    }

    fn starts_with(&self, path: &TypePath) -> bool {
        if self.parts.len() < path.parts.len() {
            false
        } else {
            self.parts[..path.parts.len()] == path.parts
        }
    }

    pub fn relative_path_to(&self, path: &TypePath) -> TypePath {
        let our_parent = self.parent();
        let their_parent = path.parent();

        let number_of_consecutively_matching_parts = izip!(&our_parent.parts, &their_parent.parts)
            .enumerate()
            .find_map(|(matches_so_far, (our_part, their_part))| {
                (our_part != their_part).then_some(matches_so_far)
            })
            .unwrap_or_else(|| min(our_parent.len(), their_parent.len()));

        let prefix = if their_parent.starts_with(&our_parent) {
            vec![ident("self")]
        } else {
            vec![ident("super"); our_parent.len() - number_of_consecutively_matching_parts]
        };

        let non_matching_path_parts = their_parent
            .take_parts()
            .into_iter()
            .skip(number_of_consecutively_matching_parts);

        let type_ident = path.ident().cloned();

        TypePath {
            parts: chain!(prefix, non_matching_path_parts, type_ident).collect(),
            is_absolute: false,
        }
    }

    pub fn parent(&self) -> TypePath {
        let parts = if self.parts.is_empty() {
            vec![]
        } else {
            self.parts[..self.parts.len() - 1].to_vec()
        };

        TypePath {
            parts,
            is_absolute: self.is_absolute,
        }
    }

    pub fn take_parts(self) -> Vec<Ident> {
        self.parts
    }

    pub fn has_multiple_parts(&self) -> bool {
        self.parts.len() > 1
    }

    fn is_absolute(path_str: &str) -> bool {
        path_str.trim_start().starts_with("::")
    }

    pub fn prepend(self, mut another: TypePath) -> Self {
        another.parts.extend(self.parts);
        another
    }

    pub fn ident(&self) -> Option<&Ident> {
        self.parts.last()
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

    #[test]
    fn path_with_two_or_more_parts_has_a_parent() {
        let type_path = TypePath::new(":: std::Type").unwrap();

        let parent = type_path.parent();

        let expected_parent = TypePath::new("::std").unwrap();
        assert_eq!(parent, expected_parent)
    }

    #[test]
    fn path_with_only_one_part_has_no_parent() {
        let type_path = TypePath::new(":: std").unwrap();

        let parent = type_path.parent();

        assert!(parent.take_parts().is_empty());
    }

    #[test]
    fn relative_path_same_mod_different_type() {
        let deeper_path = TypePath::new("a::b::SomeType").unwrap();
        let shallower_path = TypePath::new("a::b::AnotherType").unwrap();

        let relative_path = deeper_path.relative_path_to(&shallower_path);

        let expected_relative_path = TypePath::new("self::AnotherType").unwrap();
        assert_eq!(relative_path, expected_relative_path);
    }

    #[test]
    fn relative_path_both_on_root_level_different_type() {
        let deeper_path = TypePath::new("SomeType").unwrap();
        let shallower_path = TypePath::new("AnotherType").unwrap();

        let relative_path = deeper_path.relative_path_to(&shallower_path);

        let expected_relative_path = TypePath::new("self::AnotherType").unwrap();
        assert_eq!(relative_path, expected_relative_path);
    }

    #[test]
    fn relative_path_type_deeper_in() {
        let a_path = TypePath::new("a::b::SomeType").unwrap();
        let sister_path = TypePath::new("a::b::c::d::AnotherType").unwrap();

        let relative_path = a_path.relative_path_to(&sister_path);

        let expected_relative_path = TypePath::new("self::c::d::AnotherType").unwrap();
        assert_eq!(relative_path, expected_relative_path);
    }

    #[test]
    fn relative_path_type_located_few_levels_up() {
        let a_path = TypePath::new("a::b::c::SomeType").unwrap();
        let sister_path = TypePath::new("AnotherType").unwrap();

        let relative_path = a_path.relative_path_to(&sister_path);

        let expected_relative_path = TypePath::new("super::super::super::AnotherType").unwrap();
        assert_eq!(relative_path, expected_relative_path);
    }

    #[test]
    fn relative_path_up_then_down() {
        let a_path = TypePath::new("a::b::c::SomeType").unwrap();
        let sister_path = TypePath::new("d::e::AnotherType").unwrap();

        let relative_path = a_path.relative_path_to(&sister_path);

        let expected_relative_path =
            TypePath::new("super::super::super::d::e::AnotherType").unwrap();
        assert_eq!(relative_path, expected_relative_path);
    }

    #[test]
    fn path_starts_with_another() {
        let a_path = TypePath::new("a::b::c::d").unwrap();
        let prefix = TypePath::new("a::b").unwrap();

        assert!(a_path.starts_with(&prefix));
    }
    #[test]
    fn path_does_not_start_with_another() {
        let a_path = TypePath::new("a::b::c::d").unwrap();
        let prefix = TypePath::new("c::d").unwrap();

        assert!(!a_path.starts_with(&prefix));
    }

    #[test]
    fn start_with_size_guard() {
        let a_path = TypePath::new("a::b::c").unwrap();
        let prefix = TypePath::new("a::b::c::d").unwrap();

        assert!(!a_path.starts_with(&prefix));
    }
}
