use fuels_core::code_gen::abigen::Abigen;
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;

use inflector::Inflector;
use std::ops::Deref;
use std::path::Path;
use syn::parse::{Parse, ParseStream, Result as ParseResult};
use syn::{parse_macro_input, Ident, LitStr, Token};

/// Abigen proc macro definition and helper functions/types.
#[proc_macro]
pub fn abigen(input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(input as Spanned<ContractArgs>);

    Abigen::new(&args.name, &args.abi)
        .unwrap()
        .expand()
        .unwrap()
        .into()
}

#[proc_macro]
pub fn wasm_abigen(input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(input as Spanned<ContractArgs>);

    Abigen::new(&args.name, &args.abi)
        .unwrap()
        .no_std()
        .expand()
        .unwrap()
        .into()
}

/// This proc macro is used to reduce the amount of boilerplate code in integration tests.
/// When expanded, the proc macro will: launch a local provider, generate one wallet,
/// deploy the selected contract and create a contract instance with the specified name.
/// Inputs are the contract instance variable name and the test project name. For example,
/// get_contract_instance!(contract_instance, "contract_test")
/// In addition, to the contract instance you can use the variables: wallet, contract_id
#[proc_macro]
pub fn setup_contract_test(input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(input as Spanned<ContractTestArgs>);

    let abs_forc_dir = Path::new(&args.project_path)
        .canonicalize()
        .unwrap_or_else(|_| {
            panic!(
                "Unable to canonicalize forc project path: {}.",
                &args.project_path
            )
        });

    let forc_project_name = abs_forc_dir.file_name().unwrap().to_str().unwrap();

    let abi_path = abs_forc_dir
        .join(["out/debug/", forc_project_name, "-abi.json"].concat())
        .to_str()
        .unwrap()
        .to_string();

    let bin_path = abs_forc_dir
        .join(["out/debug/", forc_project_name, ".bin"].concat())
        .to_str()
        .unwrap()
        .to_string();

    let storage_path = abs_forc_dir
        .join(["out/debug/", forc_project_name, "-storage_slots.json"].concat())
        .to_str()
        .unwrap()
        .to_string();

    let contract_struct_name = &args.instance_name.to_camel_case();
    let mut abigen_token_stream: TokenStream = Abigen::new(contract_struct_name, abi_path)
        .unwrap()
        .expand()
        .unwrap()
        .into();

    let contract_instance_name = Ident::new(&args.instance_name, Span::call_site());
    let wallet_name = Ident::new(&args.wallet_name, Span::call_site());
    let contract_id_name = Ident::new(&args.id_name, Span::call_site());
    let builder_struct_name = Ident::new(
        &[contract_struct_name, "Builder"].concat(),
        Span::call_site(),
    );

    let added_token_stream: TokenStream = quote! {
        let #wallet_name = launch_provider_and_get_wallet().await;

        let #contract_id_name = Contract::deploy(
            #bin_path,
            &#wallet_name,
            TxParameters::default(),
            StorageConfiguration::with_storage_path(Some(
                #storage_path.to_string(),
            )),
        )
        .await?;

        let #contract_instance_name = #builder_struct_name::new(#contract_id_name.to_string(), #wallet_name.clone()).build();
    }
    .into();

    abigen_token_stream.extend(added_token_stream);
    abigen_token_stream
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

/// Contract procedural macro arguments.
#[cfg_attr(test, derive(Debug, Eq, PartialEq))]
pub(crate) struct ContractTestArgs {
    instance_name: String,
    wallet_name: String,
    id_name: String,
    project_path: String,
}

impl ParseInner for ContractTestArgs {
    fn spanned_parse(input: ParseStream) -> ParseResult<(Span, Self)> {
        let instance_name = input.parse::<Ident>()?.to_string();
        input.parse::<Token![,]>()?;

        let wallet_name = input.parse::<Ident>()?.to_string();
        input.parse::<Token![,]>()?;

        let id_name = input.parse::<Ident>()?.to_string();
        input.parse::<Token![,]>()?;

        let (span, project_path) = {
            let literal = input.parse::<LitStr>()?;
            (literal.span(), literal.value())
        };
        if !input.is_empty() {
            input.parse::<Token![,]>()?;
        }

        Ok((
            span,
            ContractTestArgs {
                instance_name,
                wallet_name,
                id_name,
                project_path,
            },
        ))
    }
}
