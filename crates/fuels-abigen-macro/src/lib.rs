use fuels_core::code_gen::abigen::Abigen;
use fuels_core::code_gen::flat_abigen::FlatAbigen;
use proc_macro::TokenStream;
use proc_macro2::Span;

use std::ops::Deref;
use syn::parse::{Parse, ParseStream, Result as ParseResult};
use syn::{parse_macro_input, Ident, LitStr, Token};

/// Abigen proc macro definition and helper functions/types.
#[proc_macro]
pub fn abigen(input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(input as Spanned<ContractArgs>);

    // Temporary while we support both JSON ABI formats.
    // First, we check whether it's an inline JSON string. If not, we assume it's a path.
    // For both cases, we check if it's the new flat JSON ABI format
    // or the old one.
    let is_flat =
        if args.abi.starts_with('\n') || args.abi.starts_with('{') || args.abi.starts_with('[') {
            // These keys are only found in the new JSON ABI format.
            args.abi.contains("types") && args.abi.contains("typeArguments")
        } else {
            // This is a file, not an inline JSON.
            // Check if "flat-abi" is in the file name.
            args.abi.split('/').last().unwrap().contains("flat-abi")
        };

    if is_flat {
        FlatAbigen::new(&args.name, &args.abi)
            .unwrap()
            .expand()
            .unwrap()
            .into()
    } else {
        Abigen::new(&args.name, &args.abi)
            .unwrap()
            .expand()
            .unwrap()
            .into()
    }
}

#[proc_macro]
pub fn wasm_abigen(input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(input as Spanned<ContractArgs>);

    // Temporary while we support both JSON ABI formats.
    // First, we check whether it's an inline JSON string. If not, we assume it's a path.
    // For both cases, we check if it's the new flat JSON ABI format
    // or the old one.
    let is_flat =
        if args.abi.starts_with('\n') || args.abi.starts_with('{') || args.abi.starts_with('[') {
            // These keys are only found in the new JSON ABI format.
            args.abi.contains("types") && args.abi.contains("typeArguments")
        } else {
            // This is a file, not an inline JSON.
            // Check if "flat-abi" is in the file name.
            args.abi.split('/').last().unwrap().contains("flat-abi")
        };

    if is_flat {
        FlatAbigen::new(&args.name, &args.abi)
            .unwrap()
            .no_std()
            .expand()
            .unwrap()
            .into()
    } else {
        Abigen::new(&args.name, &args.abi)
            .unwrap()
            .no_std()
            .expand()
            .unwrap()
            .into()
    }
}

/// Trait that abstracts functionality for inner data that can be parsed and
/// wrapped with a specific `Span`.
trait ParseInner: Sized {
    fn spanned_parse(input: ParseStream) -> ParseResult<(Span, Self)>;
}

impl<T: Parse> ParseInner for T {
    fn spanned_parse(input: ParseStream) -> ParseResult<(Span, Self)> {
        Ok((input.span(), T::parse(input)?))
    }
}

impl<T: ParseInner> Parse for Spanned<T> {
    fn parse(input: ParseStream) -> ParseResult<Self> {
        let (span, value) = T::spanned_parse(input)?;
        Ok(Spanned(span, value))
    }
}

/// A struct that captures `Span` information for inner parsable data.
#[cfg_attr(test, derive(Clone, Debug))]
struct Spanned<T>(Span, T);

impl<T> Spanned<T> {
    /// Retrieves the captured `Span` information for the parsed data.
    #[allow(dead_code)]
    pub fn span(&self) -> Span {
        self.0
    }

    /// Retrieves the inner data.
    #[allow(dead_code)]
    pub fn into_inner(self) -> T {
        self.1
    }
}

impl<T> Deref for Spanned<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.1
    }
}

/// Contract procedural macro arguments.
#[cfg_attr(test, derive(Debug, Eq, PartialEq))]
pub(crate) struct ContractArgs {
    name: String,
    abi: String,
}

impl ParseInner for ContractArgs {
    fn spanned_parse(input: ParseStream) -> ParseResult<(Span, Self)> {
        // read the contract name
        let name = input.parse::<Ident>()?.to_string();

        // skip the comma
        input.parse::<Token![,]>()?;

        let (span, abi) = {
            let literal = input.parse::<LitStr>()?;
            (literal.span(), literal.value())
        };
        if !input.is_empty() {
            input.parse::<Token![,]>()?;
        }

        Ok((span, ContractArgs { name, abi }))
    }
}
