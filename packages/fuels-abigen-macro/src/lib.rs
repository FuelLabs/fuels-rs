use fuels_core::{
    code_gen::abigen::{Abigen, AbigenTarget, ProgramType},
    utils::ident,
};
use parsing::{Command, MacroAbigenTargets, TestContractCommands};
use proc_macro::TokenStream;
use quote::quote;
use rand::prelude::{Rng, SeedableRng, StdRng};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};
use syn::parse_macro_input;

mod attributes;
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

struct Project {
    path: PathBuf,
}

impl Project {
    fn new(dir: &str) -> Self {
        let path = Path::new(dir).canonicalize().unwrap_or_else(|_| {
            panic!(
                "Unable to canonicalize forc project path: {}. Make sure the path is valid!",
                &dir
            )
        });

        Self { path }
    }
    fn compile_file_path(&self, suffix: &str, description: &str) -> String {
        self.path
            .join(["out/debug/", self.project_name(), suffix].concat())
            .to_str()
            .unwrap_or_else(|| panic!("could not join path for {description}"))
            .to_string()
    }

    fn project_name(&self) -> &str {
        self.path
            .file_name()
            .expect("failed to get project name")
            .to_str()
            .expect("failed to convert project name to string")
    }

    fn abi_path(&self) -> String {
        self.compile_file_path("-abi.json", "the ABI file")
    }

    fn bin_path(&self) -> String {
        self.compile_file_path(".bin", "the binary file")
    }

    fn storage_path(&self) -> String {
        self.compile_file_path("-storage_slots.json", "the storage slots file")
    }
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
    let commands = parse_macro_input!(input as TestContractCommands).commands;

    let projects: HashMap<_, _> = commands
        .iter()
        .filter_map(|c| {
            if let Command::Abigen { name, abi } = c {
                return Some((name.clone(), Project::new(abi)));
            }
            None
        })
        .collect();

    let targets: Vec<_> = projects
        .iter()
        .map(|(name, project)| AbigenTarget {
            name: name.clone(),
            abi: project.abi_path(),
            program_type: ProgramType::Contract,
        })
        .collect();

    let abigen_code = Abigen::generate(targets, false).expect("Failed to generate abigen");

    let wallet_names: Vec<_> = commands
        .iter()
        .find_map(|c| {
            if let Command::Wallets { names, .. } = c {
                return Some(names.iter().map(|wn| ident(&wn.value())).collect());
            }
            None
        })
        .unwrap_or_default();

    let num_wallets = wallet_names.len();

    let wallet_code = if !wallet_names.is_empty() {
        quote! {
            let [#(#wallet_names),*]: [_; #num_wallets] = launch_custom_provider_and_get_wallets(
                WalletsConfig::new(Some(#num_wallets as u64), None, None),
                None,
                None,
            )
            .await
            .try_into()
            .expect("Should have the exact number of wallets");

        }
    } else {
        quote! {}
    };

    let deploy_code = commands.iter().filter_map(|c| {
        if let Command::Deploy {
            name,
            contract,
            wallet,
        } = c
        {
            // Generate random salt for contract deployment
            let mut rng = StdRng::from_entropy();
            let salt: [u8; 32] = rng.gen();

            let contract_instance_name = ident(name);
            let contract_struct_name = ident(&contract.value());
            let wallet_name = ident(wallet);

            let project = projects.get(&contract.value()).expect("Should be there");
            let bin_path = project.bin_path();
            let storage_path = project.storage_path();

            return Some(quote! {
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
                    });
        }
        None
    });

    quote! {
       #abigen_code
       #wallet_code
       #(#deploy_code)*
    }
    .into()
}
