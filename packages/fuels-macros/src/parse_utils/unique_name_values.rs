use std::collections::HashMap;

use fuels_code_gen::utils::ident;
use itertools::{chain, Itertools};
use proc_macro2::{Ident, Span};
use quote::ToTokens;
use syn::{
    punctuated::Punctuated, spanned::Spanned, Error, Lit, LitStr, Meta::NameValue, MetaNameValue,
    NestedMeta, NestedMeta::Meta,
};

use crate::parse_utils::{validate_no_duplicates, ErrorsExt};

#[derive(Debug)]
pub struct UniqueNameValues {
    span: Span,
    name_values: HashMap<Ident, Lit>,
}

impl UniqueNameValues {
    pub fn new<T: ToTokens>(nested_metas: Punctuated<NestedMeta, T>) -> syn::Result<Self> {
        let span = nested_metas.span();
        let name_values = Self::extract_name_values(nested_metas.into_iter())?;

        let names = name_values.iter().map(|(name, _)| name).collect::<Vec<_>>();
        validate_no_duplicates(&names, |&&name| name.clone())?;

        Ok(Self {
            span,
            name_values: name_values.into_iter().collect(),
        })
    }

    pub fn try_get(&self, name: &str) -> Option<&Lit> {
        self.name_values.get(&ident(name))
    }

    pub fn validate_has_no_other_names(&self, allowed_names: &[&str]) -> syn::Result<()> {
        let expected_names = allowed_names
            .iter()
            .map(|name| format!("'{name}'"))
            .join(", ");

        self.name_values
            .keys()
            .filter(|name| !allowed_names.contains(&name.to_string().as_str()))
            .map(|name| {
                Error::new_spanned(
                    name.clone(),
                    format!(
                        "Attribute '{name}' not recognized! Expected one of: {expected_names}."
                    ),
                )
            })
            .validate_no_errors()
    }

    pub fn get_as_lit_str(&self, name: &str) -> syn::Result<&LitStr> {
        let value = self
            .try_get(name)
            .ok_or_else(|| Error::new(self.span, format!("Missing attribute '{name}'.")))?;

        if let Lit::Str(lit_str) = value {
            Ok(lit_str)
        } else {
            Err(Error::new_spanned(
                value.clone(),
                format!("Expected the attribute '{name}' to have a string value!"),
            ))
        }
    }

    fn extract_name_values<T: Iterator<Item = NestedMeta>>(
        nested_metas: T,
    ) -> syn::Result<Vec<(Ident, Lit)>> {
        let (name_values, name_value_errs): (Vec<_>, Vec<_>) = nested_metas
            .map(Self::extract_name_value)
            .partition_result();

        let (ident_values, name_format_errors): (Vec<_>, Vec<Error>) = name_values
            .into_iter()
            .map(|nv| {
                let ident = nv.path.get_ident().cloned().ok_or_else(|| {
                    Error::new_spanned(
                        nv.path,
                        "Attribute name cannot be a `Path` -- i.e. must not contain ':'.",
                    )
                })?;

                Ok((ident, nv.lit))
            })
            .partition_result();

        chain!(name_value_errs, name_format_errors).validate_no_errors()?;

        Ok(ident_values)
    }

    fn extract_name_value(meta: NestedMeta) -> syn::Result<MetaNameValue> {
        if let Meta(NameValue(nv)) = meta {
            Ok(nv)
        } else {
            Err(Error::new_spanned(meta, "Expected name='value'."))
        }
    }
}

#[cfg(test)]
mod tests {
    use proc_macro2::TokenStream;
    use quote::quote;
    use syn::LitBool;

    use super::*;
    use crate::parse_utils::command::Command;

    #[test]
    fn name_values_correctly_parsed() -> syn::Result<()> {
        // given
        let name_values = extract_name_values(quote! {SomeCommand(attr1="value1", attr2=true)})?;

        // when
        let attr_values = ["attr1", "attr2"].map(|attr| {
            name_values
                .try_get(attr)
                .unwrap_or_else(|| panic!("Attribute {attr} should have existed!"))
                .clone()
        });

        // then
        let expected_values = [
            Lit::Str(LitStr::new("value1", Span::call_site())),
            Lit::Bool(LitBool::new(true, Span::call_site())),
        ];

        assert_eq!(attr_values, expected_values);

        Ok(())
    }

    #[test]
    fn duplicates_cause_errors() {
        // given
        let tokens = quote! {SomeCommand(duplicate=1, something=2, duplicate=3)};

        // when
        let err = extract_name_values(tokens).expect_err("Should have failed");

        // then
        let messages = err.into_iter().map(|e| e.to_string()).collect::<Vec<_>>();
        assert_eq!(messages, vec!["Original defined here:", "Duplicate!"]);
    }

    #[test]
    fn attr_names_cannot_be_paths() {
        let tokens = quote! {SomeCommand(something::duplicate=1)};

        let err = extract_name_values(tokens).expect_err("Should have failed");

        assert_eq!(
            err.to_string(),
            "Attribute name cannot be a `Path` -- i.e. must not contain ':'."
        );
    }

    #[test]
    fn only_name_value_is_accepted() {
        let tokens = quote! {SomeCommand(name="value", "something_else")};

        let err = extract_name_values(tokens).expect_err("Should have failed");

        assert_eq!(err.to_string(), "Expected name='value'.");
    }

    #[test]
    fn validates_correct_names() -> syn::Result<()> {
        let tokens = quote! {SomeCommand(name="value", other="something_else")};
        let name_values = extract_name_values(tokens)?;

        let result = name_values.validate_has_no_other_names(&["name", "other", "another"]);

        assert!(result.is_ok());

        Ok(())
    }

    #[test]
    fn catches_incorrect_names() -> syn::Result<()> {
        let name_values =
            extract_name_values(quote! {SomeCommand(name="value", other="something_else")})?;

        let err = name_values
            .validate_has_no_other_names(&["name", "other_is_not_allowed"])
            .expect_err("Should have failed");

        assert_eq!(
            err.to_string(),
            "Attribute 'other' not recognized! Expected one of: 'name', 'other_is_not_allowed'."
        );

        Ok(())
    }

    #[test]
    fn can_get_lit_strs() -> syn::Result<()> {
        let name_values = extract_name_values(quote! {SomeCommand(name="value")})?;

        let lit_str = name_values.get_as_lit_str("name")?;

        assert_eq!(lit_str.value(), "value");

        Ok(())
    }

    #[test]
    fn cannot_get_lit_str_if_type_is_wrong() -> syn::Result<()> {
        let name_values = extract_name_values(quote! {SomeCommand(name=true)})?;

        let err = name_values
            .get_as_lit_str("name")
            .expect_err("Should have failed");

        assert_eq!(
            err.to_string(),
            "Expected the attribute 'name' to have a string value!"
        );

        Ok(())
    }

    #[test]
    fn lit_str_getter_complains_value_is_missing() -> syn::Result<()> {
        let name_values = extract_name_values(quote! {SomeCommand(name=true)})?;

        let err = name_values
            .get_as_lit_str("missing")
            .expect_err("Should have failed");

        assert_eq!(err.to_string(), "Missing attribute 'missing'.");

        Ok(())
    }

    fn extract_name_values(stream: TokenStream) -> syn::Result<UniqueNameValues> {
        let command = Command::parse_single_from_token_stream(stream)?;
        UniqueNameValues::new(command.contents)
    }
}
