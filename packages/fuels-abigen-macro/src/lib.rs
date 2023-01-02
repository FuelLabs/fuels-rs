use proc_macro::TokenStream;
use std::path::Path;

use inflector::Inflector;
use proc_macro2::Span;
use quote::quote;
use rand::prelude::{Rng, SeedableRng, StdRng};
use syn::{parse_macro_input, Ident};

use fuels_core::code_gen::abigen::{Abigen, AbigenTarget, ProgramType};
use parsing::{ContractTestArgs, MacroAbigenTargets};

mod parsing;

#[proc_macro]
pub fn abigen(input: TokenStream) -> TokenStream {
    let targets = parse_macro_input!(input as MacroAbigenTargets);

    Abigen::generate(targets.into(), false).unwrap().into()
}

#[proc_macro]
pub fn wasm_abigen(input: TokenStream) -> TokenStream {
    let targets = parse_macro_input!(input as MacroAbigenTargets);

    Abigen::generate(targets.into(), true).unwrap().into()
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
            abi: abi_path,
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
