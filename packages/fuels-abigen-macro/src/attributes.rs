use itertools::Itertools;
use proc_macro2::Span;
use syn::{
    parenthesized, parse::ParseStream, parse_macro_input::ParseMacroInput, AttributeArgs, Error,
    Lit, LitStr, Meta, MetaNameValue, NestedMeta,
};

// Used to parse the attributes inside the parentheses of a Contract, Script or
// Predicate command in the `abigen!` e.g. `abigen!(Contract(will_parse = "this_here"))`
#[derive(Debug)]
pub(crate) struct Attributes {
    content_span: Span,
    attrs: Vec<MetaNameValue>,
}

impl Attributes {
    pub(crate) fn new(input: ParseStream, valid_attr_names: &[&str]) -> syn::Result<Self> {
        let content;
        parenthesized!(content in input);

        let content_span = content.span();

        let attrs = Self::extract_attrs(&content)?;
        Self::validate_attrs(&attrs, valid_attr_names)?;

        Ok(Self {
            content_span,
            attrs,
        })
    }

    pub(crate) fn get_as_lit_str(&self, attr_name: &str) -> syn::Result<LitStr> {
        self.attrs
            .iter()
            .find(|nv| nv.path.is_ident(attr_name))
            .ok_or_else(|| {
                Error::new(
                    self.content_span,
                    format!("'{attr_name}' attribute is missing!"),
                )
            })
            .and_then(|f| match &f.lit {
                Lit::Str(lit_str) => Ok(lit_str.clone()),
                _ => Err(Error::new_spanned(
                    f,
                    format!("Expected a string for the value of the '{attr_name}' attribute."),
                )),
            })
    }

    pub(crate) fn get_as_str(&self, attr_name: &str) -> syn::Result<String> {
        self.attrs
            .iter()
            .find(|nv| nv.path.is_ident(attr_name))
            .ok_or_else(|| {
                Error::new(
                    self.content_span,
                    format!("'{attr_name}' attribute is missing!"),
                )
            })
            .and_then(|f| match &f.lit {
                Lit::Str(lit_str) => Ok(lit_str.value()),
                _ => Err(Error::new_spanned(
                    f,
                    format!("Expected a string for the value of the '{attr_name}' attribute."),
                )),
            })
    }

    fn extract_attrs(input: ParseStream) -> syn::Result<Vec<MetaNameValue>> {
        AttributeArgs::parse(input)?
            .into_iter()
            .map(|meta| match meta {
                NestedMeta::Meta(Meta::NameValue(nv)) => Ok(nv),
                _ => Err(Error::new_spanned(
                    meta,
                    "abigen! macro accepts only attributes in the form `attr = \"<value>\"`",
                )),
            })
            .collect()
    }

    fn validate_attrs(attrs: &[MetaNameValue], valid_attr_names: &[&str]) -> syn::Result<()> {
        Self::attr_names_are_valid(attrs, valid_attr_names)?;
        // must come after `attr_names_are_valid`
        Self::attr_names_are_not_duplicated(attrs)
    }

    fn attr_names_are_valid(attrs: &[MetaNameValue], valid_attr_names: &[&str]) -> syn::Result<()> {
        let has_invalid_name = |attr: &&MetaNameValue| {
            !valid_attr_names
                .iter()
                .any(|valid_name| attr.path.is_ident(&valid_name))
        };

        attrs
            .iter()
            .filter(has_invalid_name)
            .map(|invalid_attr| {
                let expected_names = valid_attr_names
                    .iter()
                    .map(|name| format!("'{name}'"))
                    .join(", ");

                Error::new_spanned(
                    invalid_attr,
                    format!("Unknown attribute! Expected one of: {expected_names}."),
                )
            })
            .reduce(|mut all_errors, current_err| {
                all_errors.combine(current_err);
                all_errors
            })
            .map(Err)
            .unwrap_or(Ok(()))
    }

    fn attr_names_are_not_duplicated(attrs: &[MetaNameValue]) -> syn::Result<()> {
        attrs
            .iter()
            .map(|arg| {
                arg.path.get_ident().expect(
                    "names to be valid since they've previously been validated to be `Ident`s.",
                )
            })
            .sorted()
            .group_by(|ident| *ident)
            .into_iter()
            .filter_map(|(name, group)| {
                let group = group.collect_vec();
                if group.len() <= 1 {
                    return None;
                }

                let mut duplicates = group.iter();
                let original = duplicates
                    .next()
                    .expect("there to be more than 1 element due to the check above");

                let err = duplicates.fold(
                    Error::new_spanned(
                        original,
                        format!("Duplicate attribute '{name}'! Original defined here:"),
                    ),
                    |mut all_errs, duplicate| {
                        all_errs.combine(Error::new_spanned(duplicate, "Duplicate: "));
                        all_errs
                    },
                );
                Some(err)
            })
            .reduce(|mut all_errs, err| {
                all_errs.combine(err);
                all_errs
            })
            .map(Err)
            .unwrap_or(Ok(()))
    }
}

#[cfg(test)]
mod tests {
    use crate::attributes::Attributes;
    use quote::quote;
    use syn::parse::ParseStream;

    #[test]
    fn fails_if_attr_names_are_not_recognized() -> syn::Result<()> {
        syn::parse::Parser::parse2(
            |input: ParseStream<'_>| {
                let err =
                    Attributes::new(input, &["name", "abi"]).expect_err("Should have failed.");

                assert_eq!(
                    err.to_string(),
                    "Unknown attribute! Expected one of: 'name', 'abi'."
                );

                Ok(())
            },
            quote! {(name = "some_value", some_typo="here")},
        )?;

        Ok(())
    }

    #[test]
    fn fails_if_name_or_abi_values_are_not_strings() -> syn::Result<()> {
        syn::parse::Parser::parse2(
            |input: ParseStream<'_>| {
                let attributes = Attributes::new(input, &["name", "abi"])?;

                {
                    let err = attributes
                        .get_as_str("name")
                        .expect_err("Should have failed.");
                    assert_eq!(
                        err.to_string(),
                        "Expected a string for the value of the 'name' attribute."
                    )
                }
                {
                    let err = attributes
                        .get_as_str("abi")
                        .expect_err("Should have failed.");
                    assert_eq!(
                        err.to_string(),
                        "Expected a string for the value of the 'abi' attribute."
                    )
                }

                Ok(())
            },
            quote! {(name = 123, abi=true)},
        )?;

        Ok(())
    }

    #[test]
    fn fails_if_names_are_duplicated() -> syn::Result<()> {
        syn::parse::Parser::parse2(
            |input: ParseStream<'_>| {
                let err =
                    Attributes::new(input, &["name", "abi"]).expect_err("Should have failed.");

                assert_eq!(
                    err.to_string(),
                    "Duplicate attribute 'name'! Original defined here:"
                );

                Ok(())
            },
            quote! {(name = "something", abi="else", name="something")},
        )?;

        Ok(())
    }

    #[test]
    fn can_extract_attr_values() -> syn::Result<()> {
        syn::parse::Parser::parse2(
            |input: ParseStream<'_>| {
                let sut = Attributes::new(input, &["name", "abi"])?;

                {
                    let value = sut.get_as_str("name").unwrap();

                    assert_eq!(value, "some_value");
                }
                {
                    let value = sut.get_as_str("abi").unwrap();

                    assert_eq!(value, "some_abi");
                }

                Ok(())
            },
            quote! {(name = "some_value", abi = "some_abi")},
        )
    }
}
