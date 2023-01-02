use fuels_core::code_gen::abigen::{Abigen, AbigenTarget, ProgramType};
use inflector::Inflector;
use itertools::Itertools;
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use rand::prelude::{Rng, SeedableRng, StdRng};

use std::path::Path;

use syn::parse_macro_input::ParseMacroInput;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{
    parenthesized,
    parse::{Parse, ParseStream, Result as ParseResult},
    parse_macro_input, AttributeArgs, Ident, Lit, LitStr, Meta, MetaNameValue, NestedMeta, Token,
};

/// Abigen proc macro definition and helper functions/types.
#[proc_macro]
pub fn abigen(input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(input as MultipleAbis);

    let targets = into_abigen_targets(args, ProgramType::Contract);

    Abigen::generate(targets, false).unwrap().into()
}

#[proc_macro]
pub fn new_abigen(input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(input as NewMultipleAbis);

    let targets = new_into_abigen_targets(args);

    Abigen::generate(targets, false).unwrap().into()
}
// abigen!(
//             Contract(name=MyContract, abi="packages/fuels/tests/storage/contract_storage_test/out/debug/contract_storage_test-abi.json"),
//             Script(name=MyContract, abi="packages/fuels/tests/storage/contract_storage_test/out/debug/contract_storage_test-abi.json"),
//             Predicate(name=MyContract, abi="packages/fuels/tests/storage/contract_storage_test/out/debug/contract_storage_test-abi.json")
//     );

/// Abigen proc macro definition and helper functions/types for scripts
#[proc_macro]
pub fn script_abigen(input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(input as MultipleAbis);

    let targets = into_abigen_targets(args, ProgramType::Script);

    Abigen::generate(targets, false).unwrap().into()
}

/// Abigen proc macro definition and helper functions/types for scripts
#[proc_macro]
pub fn predicate_abigen(input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(input as MultipleAbis);

    let targets = into_abigen_targets(args, ProgramType::Predicate);

    Abigen::generate(targets, false).unwrap().into()
}

#[proc_macro]
pub fn wasm_abigen(input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(input as MultipleAbis);

    let targets = into_abigen_targets(args, ProgramType::Contract);

    Abigen::generate(targets, true).unwrap().into()
}

fn into_abigen_targets(args: MultipleAbis, program_type: ProgramType) -> Vec<AbigenTarget> {
    args.abis
        .into_iter()
        .map(|abi_details| AbigenTarget {
            name: abi_details.name,
            source: abi_details.source,
            program_type,
        })
        .collect()
}

fn new_into_abigen_targets(args: NewMultipleAbis) -> Vec<AbigenTarget> {
    args.abis
        .into_iter()
        .map(|abi_details| AbigenTarget {
            name: abi_details.name,
            source: abi_details.abi,
            program_type: abi_details.program_type,
        })
        .collect()
}

/// This proc macro is used to reduce the amount of boilerplate code in integration tests.
/// When expanded, the proc macro will: launch a local provider, generate one wallet,
/// deploy the selected contract and create a contract instance.
/// The names for the contract instance and wallet variables must be provided as inputs.
/// This macro can be called multiple times inside a function if the variables names are changed.
/// The same contract can be deployed multiple times as the macro uses deployment with salt.
/// However, if you need to have a shared wallet between macros, the first macro must set the
/// wallet name to `wallet`. The other ones must set the wallet name to `None`.
#[proc_macro]
pub fn setup_contract_test(input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(input as ContractTestArgs);

    let abs_forc_dir = Path::new(&args.project_path)
        .canonicalize()
        .unwrap_or_else(|_| {
            panic!(
                "Unable to canonicalize forc project path: {}. Make sure the path is valid!",
                &args.project_path
            )
        });

    let forc_project_name = abs_forc_dir
        .file_name()
        .expect("failed to get project name")
        .to_str()
        .expect("failed to convert project name to string");

    let compiled_file_path = |suffix: &str, desc: &str| {
        abs_forc_dir
            .join(["out/debug/", forc_project_name, suffix].concat())
            .to_str()
            .unwrap_or_else(|| panic!("could not join path for {desc}"))
            .to_string()
    };

    let abi_path = compiled_file_path("-abi.json", "the ABI file");
    let bin_path = compiled_file_path(".bin", "the binary file");
    let storage_path = compiled_file_path("-storage_slots.json", "the storage slots file");

    let contract_struct_name = args.instance_name.to_class_case();
    let mut abigen_token_stream: TokenStream = Abigen::generate(
        vec![AbigenTarget {
            name: contract_struct_name.clone(),
            source: abi_path,
            program_type: ProgramType::Contract,
        }],
        false,
    )
    .unwrap()
    .into();

    // Generate random salt for contract deployment
    let mut rng = StdRng::from_entropy();
    let salt: [u8; 32] = rng.gen();

    let contract_instance_name = Ident::new(&args.instance_name, Span::call_site());
    let contract_struct_name = Ident::new(&contract_struct_name, Span::call_site());

    // If the wallet name is None, do not launch a new provider and use the default `wallet` name
    let (wallet_name, wallet_token_stream): (Ident, TokenStream) = if args.wallet_name == "None" {
        (Ident::new("wallet", Span::call_site()), quote! {}.into())
    } else {
        let wallet_name = Ident::new(&args.wallet_name, Span::call_site());
        (
            wallet_name.clone(),
            quote! {let #wallet_name = launch_provider_and_get_wallet().await;}.into(),
        )
    };

    let contract_deploy_token_stream: TokenStream = quote! {
        let #contract_instance_name = #contract_struct_name::new(
            Contract::deploy_with_parameters(
                #bin_path,
                &#wallet_name,
                TxParameters::default(),
                StorageConfiguration::with_storage_path(Some(
                    #storage_path.to_string(),
                )),
                Salt::from([#(#salt),*]),
            )
            .await
            .expect("Failed to deploy the contract"),
            #wallet_name.clone(),
        );
    }
    .into();

    abigen_token_stream.extend(wallet_token_stream);
    abigen_token_stream.extend(contract_deploy_token_stream);
    abigen_token_stream
}

#[derive(Clone)]
struct Abi {
    name: String,
    source: String,
}

#[derive(Clone)]
struct MultipleAbis {
    abis: Vec<Abi>,
}

#[derive(Clone)]
struct NewAbi {
    name: String,
    abi: String,
    program_type: ProgramType,
}

#[derive(Clone)]
struct NewMultipleAbis {
    abis: Punctuated<NewAbi, Token![,]>,
}

impl Parse for MultipleAbis {
    fn parse(input: ParseStream) -> ParseResult<Self> {
        let abis = input
            .parse_terminated::<_, Token![,]>(ParseMacroInput::parse)?
            .into_iter()
            .collect::<Vec<_>>();

        Ok(MultipleAbis { abis })
    }
}

impl Parse for NewMultipleAbis {
    fn parse(input: ParseStream) -> ParseResult<Self> {
        let abis = input.parse_terminated::<_, Token![,]>(ParseMacroInput::parse)?;

        Ok(Self { abis })
    }
}

fn parse_program_type(input: ParseStream) -> ParseResult<ProgramType> {
    let ident = input.parse::<Ident>()?;

    match ident.to_string().as_ref() {
        "Contract" => Ok(ProgramType::Contract),
        "Script" => Ok(ProgramType::Script),
        "Predicate" => Ok(ProgramType::Predicate),
        _ => Err(syn::Error::new_spanned(
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
            _ => Err(syn::Error::new_spanned(
                e,
                "abigen macro accepts only attributes in the form `attr = \"<value>\"`",
            )),
        })
        .collect::<Result<Vec<_>, _>>()
}

fn validate_args_names_valid(args: &[MetaNameValue]) -> syn::Result<()> {
    let valid_attr_names = ["name", "abi", "program_type"];

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

            syn::Error::new_spanned(
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
                    syn::Error::new_spanned(original, "Duplicate arguments! Original: "),
                    |mut all_errs, duplicate| {
                        all_errs.combine(syn::Error::new_spanned(duplicate, "Duplicate: "));
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

impl Parse for NewAbi {
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
        let name = attr_value(&attrs, "name", content_start)?.value();
        let abi = attr_value(&attrs, "abi", content_start)?.value();

        Ok(Self {
            name,
            abi,
            program_type,
        })
    }
}

fn attr_value(args: &[MetaNameValue], attr_name: &str, content_start: Span) -> ParseResult<LitStr> {
    args.iter()
        .find(|nv| nv.path.is_ident(attr_name))
        .ok_or_else(|| {
            syn::Error::new(
                content_start,
                format!("'{attr_name}' attribute is missing!"),
            )
        })
        .and_then(|f| match &f.lit {
            Lit::Str(lit_str) => Ok(lit_str.clone()),
            _ => Err(syn::Error::new_spanned(
                f,
                format!("Expected a string for the '{attr_name}' attribute"),
            )),
        })
}

impl Parse for Abi {
    fn parse(input: ParseStream) -> ParseResult<Self> {
        let name = input.parse::<Ident>()?.to_string();

        // skip the comma
        input.parse::<Token![,]>()?;

        let abi = input.parse::<LitStr>()?.value();

        Ok(Abi { name, source: abi })
    }
}

/// Contract procedural macro arguments.
#[cfg_attr(test, derive(Debug, Eq, PartialEq))]
struct ContractTestArgs {
    instance_name: String,
    wallet_name: String,
    project_path: String,
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
