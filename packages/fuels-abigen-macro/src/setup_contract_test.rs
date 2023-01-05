use crate::parsing::{Command, TestContractCommands};
use fuels_core::{
    code_gen::abigen::{Abigen, AbigenTarget, ProgramType},
    utils::ident,
};
use proc_macro2::{Ident, TokenStream};
use quote::quote;
use rand::prelude::{Rng, SeedableRng, StdRng};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

pub(crate) fn generate_setup_contract_test_code(
    test_contarct_commands: TestContractCommands,
) -> TokenStream {
    let commands = test_contarct_commands.commands;

    let project_lookup = generate_project_lookup(&commands);
    let abigen_code = abigen_code(&project_lookup);
    let wallet_code = wallet_initialization_code(&commands);
    let deploy_code = contract_deploying_code(&commands, &project_lookup);

    quote! {
       #abigen_code
       #wallet_code
       #deploy_code
    }
}

fn generate_project_lookup(commands: &[Command]) -> HashMap<String, Project> {
    commands
        .iter()
        .filter_map(|c| {
            if let Command::Abigen { name, abi } = c {
                return Some((name.clone(), Project::new(abi)));
            }
            None
        })
        .collect()
}

fn generate_abigen_targets(project_lookup: &HashMap<String, Project>) -> Vec<AbigenTarget> {
    project_lookup
        .iter()
        .map(|(name, project)| AbigenTarget {
            name: name.clone(),
            abi: project.abi_path(),
            program_type: ProgramType::Contract,
        })
        .collect()
}

fn abigen_code(project_lookup: &HashMap<String, Project>) -> TokenStream {
    let targets = generate_abigen_targets(project_lookup);
    Abigen::generate(targets, false).expect("Failed to generate abigen")
}

fn extract_wallet_names(commands: &[Command]) -> Vec<Ident> {
    commands
        .iter()
        .find_map(|c| {
            if let Command::Wallets { names, .. } = c {
                return Some(names.iter().map(|wn| ident(&wn.value())).collect());
            }
            None
        })
        .unwrap_or_default()
}

fn wallet_initialization_code(commands: &[Command]) -> TokenStream {
    let wallet_names = extract_wallet_names(commands);

    if wallet_names.is_empty() {
        return quote! {};
    }

    let num_wallets = wallet_names.len();
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
}

fn contract_deploying_code(
    commands: &[Command],
    project_lookup: &HashMap<String, Project>,
) -> TokenStream {
    commands
        .iter()
        .filter_map(|command| {
            if let Command::Deploy {
                name,
                contract,
                wallet,
            } = command
            {
                return Some((name, contract, wallet));
            }
            None
        })
        .map(|(name, contract, wallet)| {
            // Generate random salt for contract deployment
            let mut rng = StdRng::from_entropy();
            let salt: [u8; 32] = rng.gen();

            let contract_instance_name = ident(name);
            let contract_struct_name = ident(&contract.value());
            let wallet_name = ident(wallet);

            let project = project_lookup
                .get(&contract.value())
                .expect("Project should be in lookup");
            let bin_path = project.bin_path();
            let storage_path = project.storage_path();

            quote! {
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
        })
        .reduce(|mut all_code, code| {
            all_code.extend(code);
            all_code
        })
        .unwrap_or_default()
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
