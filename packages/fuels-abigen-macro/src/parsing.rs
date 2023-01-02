use itertools::Itertools;
use proc_macro2::{Ident, Span};
use syn::parse::{Parse, ParseStream};
use syn::parse_macro_input::ParseMacroInput;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{
    parenthesized, AttributeArgs, Error, Lit, LitStr, Meta, MetaNameValue, NestedMeta,
    Result as ParseResult, Token,
};

use fuels_core::code_gen::abigen::{AbigenTarget, ProgramType};

impl From<MacroAbigenTargets> for Vec<AbigenTarget> {
    fn from(targets: MacroAbigenTargets) -> Self {
        targets
            .targets
            .into_iter()
            .map(
                |MacroAbigenTarget {
                     name,
                     abi,
                     program_type,
                 }| AbigenTarget {
                    name,
                    abi,
                    program_type,
                },
            )
            .collect()
    }
}

struct MacroAbigenTarget {
    name: String,
    abi: String,
    program_type: ProgramType,
}

pub(crate) struct MacroAbigenTargets {
    targets: Punctuated<MacroAbigenTarget, Token![,]>,
}

fn parse_program_type(input: ParseStream) -> ParseResult<ProgramType> {
    let ident = input.parse::<Ident>()?;

    match ident.to_string().as_ref() {
        "Contract" => Ok(ProgramType::Contract),
        "Script" => Ok(ProgramType::Script),
        "Predicate" => Ok(ProgramType::Predicate),
        _ => Err(Error::new_spanned(
            ident,
            "Unsupported program type. Expected: 'Contract', 'Script' or 'Predicate'",
        )),
    }
}

fn extract_attrs(input: ParseStream) -> syn::Result<Vec<MetaNameValue>> {
    AttributeArgs::parse(input)?
        .into_iter()
        .map(|e| match e {
            NestedMeta::Meta(Meta::NameValue(nv)) => Ok(nv),
            _ => Err(Error::new_spanned(
                e,
                "abigen macro accepts only attributes in the form `attr = \"<value>\"`",
            )),
        })
        .collect::<Result<Vec<_>, _>>()
}

fn validate_args_names_valid(args: &[MetaNameValue]) -> syn::Result<()> {
    let valid_attr_names = ["name", "abi", "program_type", "no_std"];

    args.iter()
        .filter(|arg| {
            !valid_attr_names
                .iter()
                .any(|valid_name| arg.path.is_ident(valid_name))
        })
        .map(|invalid_nv| {
            let expected_names = valid_attr_names
                .iter()
                .map(|name| format!("'{name}'"))
                .join(",");

            Error::new_spanned(
                &invalid_nv.path,
                format!("Unknown attribute, expected one of: [{expected_names}]"),
            )
        })
        .reduce(|mut all_errors, current_err| {
            all_errors.combine(current_err);
            all_errors
        })
        .map(Err)
        .unwrap_or(Ok(()))
}

fn validate_no_duplicates(args: &[MetaNameValue]) -> syn::Result<()> {
    args.iter()
        .map(|arg| {
            arg.path
                .get_ident()
                .expect("Previously validated that the names were valid")
        })
        .sorted()
        .group_by(|ident| *ident)
        .into_iter()
        .filter_map(|(_, group)| {
            let group = group.collect_vec();

            if group.len() > 1 {
                let mut duplicates = group.iter();
                let original = duplicates.next().expect("We know there is > 1 element");

                let err = duplicates.fold(
                    Error::new_spanned(original, "Duplicate arguments! Original: "),
                    |mut all_errs, duplicate| {
                        all_errs.combine(Error::new_spanned(duplicate, "Duplicate: "));
                        all_errs
                    },
                );
                Some(err)
            } else {
                None
            }
        })
        .reduce(|mut all_errs, err| {
            all_errs.combine(err);
            all_errs
        })
        .map(Err)
        .unwrap_or(Ok(()))
}

fn validate_args(args: &[MetaNameValue]) -> syn::Result<()> {
    [
        validate_args_names_valid(args),
        validate_no_duplicates(args),
    ]
    .into_iter()
    .filter_map(Result::err)
    .reduce(|mut all_errs, err| {
        all_errs.combine(err);
        all_errs
    })
    .map(Err)
    .unwrap_or(Ok(()))
}

impl Parse for MacroAbigenTargets {
    fn parse(input: ParseStream) -> ParseResult<Self> {
        let abis = input.parse_terminated(ParseMacroInput::parse)?;

        Ok(Self { targets: abis })
    }
}

impl Parse for MacroAbigenTarget {
    fn parse(input: ParseStream) -> ParseResult<Self> {
        let program_type = parse_program_type(input)?;

        let content;
        parenthesized!(content in input);

        let attrs = extract_attrs(&content)?;
        validate_args(&attrs)?;

        let content_start = attrs
            .first()
            .map(|f| f.span())
            .unwrap_or_else(|| content.span());

        let name = attr_raw_str_value(&attrs, "name", content_start)?.value();
        let abi = attr_raw_str_value(&attrs, "abi", content_start)?.value();

        Ok(Self {
            name,
            abi,
            program_type,
        })
    }
}

fn attr_raw_str_value(
    args: &[MetaNameValue],
    attr_name: &str,
    content_start: Span,
) -> ParseResult<LitStr> {
    args.iter()
        .find(|nv| nv.path.is_ident(attr_name))
        .ok_or_else(|| {
            Error::new(
                content_start,
                format!("'{attr_name}' attribute is missing!"),
            )
        })
        .and_then(|f| match &f.lit {
            Lit::Str(lit_str) => Ok(lit_str.clone()),
            _ => Err(Error::new_spanned(
                f,
                format!("Expected a string for the '{attr_name}' attribute"),
            )),
        })
}

/// Contract procedural macro arguments.
#[cfg_attr(test, derive(Debug, Eq, PartialEq))]
pub(crate) struct ContractTestArgs {
    pub(crate) instance_name: String,
    pub(crate) wallet_name: String,
    pub(crate) project_path: String,
}

impl Parse for ContractTestArgs {
    fn parse(input: ParseStream) -> ParseResult<Self> {
        let instance_name = input.parse::<Ident>()?.to_string();
        input.parse::<Token![,]>()?;

        let wallet_name = input.parse::<Ident>()?.to_string();
        input.parse::<Token![,]>()?;

        let (_, project_path) = {
            let literal = input.parse::<LitStr>()?;
            (literal.span(), literal.value())
        };
        if !input.is_empty() {
            input.parse::<Token![,]>()?;
        }

        Ok(ContractTestArgs {
            instance_name,
            wallet_name,
            project_path,
        })
    }
}
