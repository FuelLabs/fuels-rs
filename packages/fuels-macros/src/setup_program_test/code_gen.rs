use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use fuels_code_gen::{utils::ident, Abi, Abigen, AbigenTarget, ProgramType};
use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use syn::LitStr;

use crate::setup_program_test::parsing::{
    AbigenCommand, BuildProfile, DeployContractCommand, InitializeWalletCommand, LoadScriptCommand,
    SetOptionsCommand, TestProgramCommands,
};

pub(crate) fn generate_setup_program_test_code(
    commands: TestProgramCommands,
) -> syn::Result<TokenStream> {
    let TestProgramCommands {
        set_options,
        initialize_wallets,
        generate_bindings,
        deploy_contract,
        load_scripts,
    } = commands;

    let SetOptionsCommand { profile } = set_options.unwrap_or_default();
    let project_lookup = generate_project_lookup(&generate_bindings, profile)?;
    let abigen_code = abigen_code(&project_lookup)?;
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

fn generate_project_lookup(
    commands: &AbigenCommand,
    profile: BuildProfile,
) -> syn::Result<HashMap<String, Project>> {
    let pairs = commands
        .targets
        .iter()
        .map(|command| -> syn::Result<_> {
            let project = Project::new(command.program_type, &command.project, profile.clone())?;
            Ok((command.name.value(), project))
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(pairs.into_iter().collect())
}

fn abigen_code(project_lookup: &HashMap<String, Project>) -> syn::Result<TokenStream> {
    let targets = parse_abigen_targets(project_lookup)?;

    Ok(Abigen::generate(targets, false).expect("abigen generation failed"))
}

fn parse_abigen_targets(
    project_lookup: &HashMap<String, Project>,
) -> syn::Result<Vec<AbigenTarget>> {
    project_lookup
        .iter()
        .map(|(name, project)| {
            let source = Abi::load_from(project.abi_path())
                .map_err(|e| syn::Error::new(project.path_span, e.to_string()))?;

            Ok(AbigenTarget::new(
                name.clone(),
                source,
                project.program_type,
            ))
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
        let [#(#wallet_names),*]: [_; #num_wallets] = ::fuels::test_helpers::launch_custom_provider_and_get_wallets(
            ::fuels::test_helpers::WalletsConfig::new(Some(#num_wallets as u64), None, None),
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
            let contract_instance_name = ident(&command.name);
            let contract_struct_name = ident(&command.contract.value());
            let wallet_name = ident(&command.wallet);
            let random_salt = command.random_salt;

            let project = project_lookup
                .get(&command.contract.value())
                .expect("Project should be in lookup");
            let bin_path = project.bin_path();

            let salt = if random_salt {
                quote! {
                    // Generate random salt for contract deployment.
                    // These lines must be inside the `quote!` macro, otherwise the salt remains
                    // identical between macro compilation, causing contract id collision.
                    ::fuels::test_helpers::generate_random_salt()
                }
            } else {
                quote! { [0; 32] }
            };

            quote! {
                let salt: [u8; 32] = #salt;

                let #contract_instance_name = {
                    let load_config = ::fuels::programs::contract::LoadConfiguration::default().with_salt(salt);

                    let loaded_contract = ::fuels::programs::contract::Contract::load_from(
                        #bin_path,
                        load_config
                    )
                    .expect("Failed to load the contract");

                    let contract_id = loaded_contract.deploy_if_not_exists(
                        &#wallet_name,
                        ::fuels::types::transaction::TxPolicies::default()
                    )
                    .await
                    .expect("Failed to deploy the contract");

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
    path_span: Span,
    profile: BuildProfile,
}

impl Project {
    fn new(program_type: ProgramType, dir: &LitStr, profile: BuildProfile) -> syn::Result<Self> {
        let path = Path::new(&dir.value()).canonicalize().map_err(|_| {
            syn::Error::new_spanned(
                dir.clone(),
                "unable to canonicalize forc project path. Make sure the path is valid!",
            )
        })?;

        Ok(Self {
            program_type,
            path,
            path_span: dir.span(),
            profile,
        })
    }

    fn compile_file_path(&self, suffix: &str, description: &str) -> String {
        self.path
            .join(
                [
                    format!("out/{}/", &self.profile).as_str(),
                    self.project_name(),
                    suffix,
                ]
                .concat(),
            )
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
