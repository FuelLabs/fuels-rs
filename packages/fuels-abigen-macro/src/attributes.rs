use itertools::Itertools;
use proc_macro2::Span;
use syn::parse::{Parse, ParseStream};
use syn::parse_macro_input::ParseMacroInput;
use syn::spanned::Spanned;
use syn::{
    parenthesized, AttributeArgs, Error, Lit, Meta, MetaNameValue, NestedMeta,
    Result as ParseResult,
};

pub(crate) struct Attributes {
    content_start: Span,
    attrs: Vec<MetaNameValue>,
}

impl Parse for Attributes {
    fn parse(input: ParseStream) -> ParseResult<Self> {
        let content;
        parenthesized!(content in input);

        let attrs = Self::extract_attrs(&content)?;
        Self::validate_attrs(&attrs)?;

        let content_start = attrs
            .first()
            .map(|f| f.span())
            .unwrap_or_else(|| content.span());

        Ok(Self {
            content_start,
            attrs,
        })
    }
}

impl Attributes {
    pub(crate) fn get_as_str(&self, attr_name: &str) -> syn::Result<String> {
        self.attrs
            .iter()
            .find(|nv| nv.path.is_ident(attr_name))
            .ok_or_else(|| {
                Error::new(
                    self.content_start,
                    format!("'{attr_name}' attribute is missing!"),
                )
            })
            .and_then(|f| match &f.lit {
                Lit::Str(lit_str) => Ok(lit_str.value()),
                _ => Err(Error::new_spanned(
                    f,
                    format!("Expected a string for the '{attr_name}' attribute"),
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

    fn validate_attrs(attrs: &[MetaNameValue]) -> syn::Result<()> {
        Self::validate_names_valid(attrs)?;
        // must come after `validate_names_valid`
        Self::validate_no_duplicates(attrs)
    }

    fn validate_names_valid(attrs: &[MetaNameValue]) -> syn::Result<()> {
        let valid_attr_names = ["name", "abi", "program_type", "no_std"];
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
                    .join(",");

                Error::new_spanned(
                    invalid_attr,
                    format!("Unknown attribute, expected one of: {expected_names}."),
                )
            })
            .reduce(|mut all_errors, current_err| {
                all_errors.combine(current_err);
                all_errors
            })
            .map(Err)
            .unwrap_or(Ok(()))
    }

    fn validate_no_duplicates(attrs: &[MetaNameValue]) -> syn::Result<()> {
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
            .filter_map(|(_, group)| {
                let group = group.collect_vec();
                if group.len() <= 1 {
                    return None;
                }

                let mut duplicates = group.iter();
                let original = duplicates
                    .next()
                    .expect("there to be more than 1 element due to the check above");

                let err = duplicates.fold(
                    Error::new_spanned(original, "Duplicate arguments! Original: "),
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
