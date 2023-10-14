use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use fuels_code_gen::{utils::ident, Abigen, AbigenTarget, ProgramType};
use proc_macro2::{Ident, TokenStream};
use quote::quote;
use rand::{rngs::StdRng, Rng, SeedableRng};
use syn::LitStr;

use crate::setup_program_test::parsing::{
    AbigenCommand, DeployContractCommand, InitializeWalletCommand, LoadScriptCommand,
    TestProgramCommands,
};

pub(crate) fn generate_setup_program_test_code(
    commands: TestProgramCommands,
) -> syn::Result<TokenStream> {
    let TestProgramCommands {
        initialize_wallets,
        generate_bindings,
        deploy_contract,
        load_scripts,
    } = commands;

    let project_lookup = generate_project_lookup(&generate_bindings)?;
    let abigen_code = abigen_code(&project_lookup);
    let wallet_code = wallet_initialization_code(initialize_wallets);
    let deploy_code = contract_deploying_code(&deploy_contract, &project_lookup);
    let script_code = script_loading_code(&load_scripts, &project_lookup);

    Ok(quote! {
       #abigen_code
       #wallet_code
       #deploy_code
       #script_code
    })
}

fn generate_project_lookup(commands: &AbigenCommand) -> syn::Result<HashMap<String, Project>> {
    let pairs = commands
        .targets
        .iter()
        .map(|command| -> syn::Result<_> {
            let project = Project::new(command.program_type, &command.project)?;
            Ok((command.name.value(), project))
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(pairs.into_iter().collect())
}

fn abigen_code(project_lookup: &HashMap<String, Project>) -> TokenStream {
    let targets = generate_abigen_targets(project_lookup);
    Abigen::generate(targets, false).expect("Failed to generate abigen")
}

fn generate_abigen_targets(project_lookup: &HashMap<String, Project>) -> Vec<AbigenTarget> {
    project_lookup
        .iter()
        .map(|(name, project)| AbigenTarget {
            name: name.clone(),
            abi: project.abi_path(),
            program_type: project.program_type,
        })
        .collect()
}

fn wallet_initialization_code(maybe_command: Option<InitializeWalletCommand>) -> TokenStream {
    let command = if let Some(command) = maybe_command {
        command
    } else {
        return Default::default();
    };

    let wallet_names = extract_wallet_names(&command);

    if wallet_names.is_empty() {
        return Default::default();
    }

    let num_wallets = wallet_names.len();
    quote! {
        let [#(#wallet_names),*]: [_; #num_wallets] = launch_custom_provider_and_get_wallets(
            WalletsConfig::new(Some(#num_wallets as u64), None, None),
            None,
            None,
        )
        .await
        .expect("Error while trying to fetch wallets from the custom provider")
        .try_into()
        .expect("Should have the exact number of wallets");
    }
}

fn extract_wallet_names(command: &InitializeWalletCommand) -> Vec<Ident> {
    command
        .names
        .iter()
        .map(|name| ident(&name.value()))
        .collect()
}

fn contract_deploying_code(
    commands: &[DeployContractCommand],
    project_lookup: &HashMap<String, Project>,
) -> TokenStream {
    commands
        .iter()
        .map(|command| {
            // Generate random salt for contract deployment
            let mut rng = StdRng::from_entropy();
            let salt: [u8; 32] = rng.gen();

            let contract_instance_name = ident(&command.name);
            let contract_struct_name = ident(&command.contract.value());
            let wallet_name = ident(&command.wallet);

            let project = project_lookup
                .get(&command.contract.value())
                .expect("Project should be in lookup");
            let bin_path = project.bin_path();

            quote! {
                let #contract_instance_name = {
                    let load_config = LoadConfiguration::default().with_salt([#(#salt),*]);

                    let loaded_contract = Contract::load_from(#bin_path, load_config).expect("Failed to load the contract");

                    let contract_id = loaded_contract.deploy(&#wallet_name, TxParameters::default()).await.expect("Failed to deploy the contract");
                    #contract_struct_name::new(contract_id, #wallet_name.clone())
                };
            }
        })
        .reduce(|mut all_code, code| {
            all_code.extend(code);
            all_code
        })
        .unwrap_or_default()
}

fn script_loading_code(
    commands: &[LoadScriptCommand],
    project_lookup: &HashMap<String, Project>,
) -> TokenStream {
    commands
        .iter()
        .map(|command| {
            let script_instance_name = ident(&command.name);
            let script_struct_name = ident(&command.script.value());
            let wallet_name = ident(&command.wallet);

            let project = project_lookup
                .get(&command.script.value())
                .expect("Project should be in lookup");
            let bin_path = project.bin_path();

            quote! {
                let #script_instance_name = #script_struct_name::new(#wallet_name.clone(), #bin_path);
            }
        })
        .reduce(|mut all_code, code| {
            all_code.extend(code);
            all_code
        })
        .unwrap_or_default()
}

struct Project {
    program_type: ProgramType,
    path: PathBuf,
}

impl Project {
    fn new(program_type: ProgramType, dir: &LitStr) -> syn::Result<Self> {
        let path = Path::new(&dir.value()).canonicalize().map_err(|_| {
            syn::Error::new_spanned(
                dir.clone(),
                "Unable to canonicalize forc project path. Make sure the path is valid!",
            )
        })?;

        Ok(Self { program_type, path })
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
}
