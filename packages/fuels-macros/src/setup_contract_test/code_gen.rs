use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use proc_macro2::{Ident, TokenStream};
use quote::quote;
use rand::{prelude::StdRng, Rng, SeedableRng};
use syn::LitStr;

use crate::{
    abigen::{Abigen, AbigenTarget, ProgramType},
    setup_contract_test::parsing::{
        DeployContract, GenerateContract, InitializeWallet, TestContractCommands,
    },
    utils::ident,
};

pub(crate) fn generate_setup_contract_test_code(
    commands: TestContractCommands,
) -> syn::Result<TokenStream> {
    let TestContractCommands {
        initialize_wallets,
        generate_contract,
        deploy_contract,
    } = commands;

    let project_lookup = generate_project_lookup(&generate_contract)?;

    let abigen_code = abigen_code(&project_lookup);

    let wallet_code = wallet_initialization_code(initialize_wallets);

    let deploy_code = contract_deploying_code(&deploy_contract, &project_lookup);

    Ok(quote! {
       #abigen_code
       #wallet_code
       #deploy_code
    })
}

fn generate_project_lookup(commands: &[GenerateContract]) -> syn::Result<HashMap<String, Project>> {
    let pairs = commands
        .iter()
        .map(|command| -> syn::Result<_> {
            let project = Project::new(&command.abi)?;
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
            program_type: ProgramType::Contract,
        })
        .collect()
}

fn wallet_initialization_code(maybe_command: Option<InitializeWallet>) -> TokenStream {
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
        .try_into()
        .expect("Should have the exact number of wallets");
    }
}

fn extract_wallet_names(command: &InitializeWallet) -> Vec<Ident> {
    command
        .names
        .iter()
        .map(|name| ident(&name.value()))
        .collect()
}

fn contract_deploying_code(
    commands: &[DeployContract],
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
    fn new(dir: &LitStr) -> syn::Result<Self> {
        let path = Path::new(&dir.value()).canonicalize().map_err(|_| {
            syn::Error::new_spanned(
                dir.clone(),
                "Unable to canonicalize forc project path. Make sure the path is valid!",
            )
        })?;

        Ok(Self { path })
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
