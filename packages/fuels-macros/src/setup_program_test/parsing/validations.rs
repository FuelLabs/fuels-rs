use std::collections::HashSet;

use fuels_code_gen::ProgramType;
use proc_macro2::Span;
use syn::{Error, LitStr, Result};

use crate::{
    parse_utils::ErrorsExt,
    setup_program_test::parsing::{
        AbigenCommand, DeployContractCommand, InitializeWalletCommand, LoadScriptCommand,
    },
};

pub(crate) fn extract_the_abigen_command(
    parent_span: Span,
    abigen_commands: &[AbigenCommand],
) -> Result<AbigenCommand> {
    match abigen_commands {
        [single_command] => Ok(single_command.clone()),
        commands => {
            let err = commands
                .iter()
                .map(|command| Error::new(command.span, "Only one `Abigen` command allowed"))
                .combine_errors()
                .unwrap_or_else(|| Error::new(parent_span, "Add an `Abigen(..)` command!"));

            Err(err)
        }
    }
}

pub(crate) fn validate_all_contracts_are_known(
    abigen_command: &AbigenCommand,
    deploy_commands: &[DeployContractCommand],
) -> Result<()> {
    extract_contracts_to_deploy(deploy_commands)
        .difference(&names_of_program_bindings(
            abigen_command,
            ProgramType::Contract,
        ))
        .flat_map(|unknown_contract| {
            [
                Error::new_spanned(unknown_contract, "Contract is unknown"),
                Error::new(
                    abigen_command.span,
                    format!(
                        "Consider adding: Contract(name=\"{}\", project=...)",
                        unknown_contract.value()
                    ),
                ),
            ]
        })
        .validate_no_errors()
}

pub(crate) fn validate_all_scripts_are_known(
    abigen_command: &AbigenCommand,
    load_commands: &[LoadScriptCommand],
) -> Result<()> {
    extract_scripts_to_load(load_commands)
        .difference(&names_of_program_bindings(
            abigen_command,
            ProgramType::Script,
        ))
        .flat_map(|unknown_contract| {
            [
                Error::new_spanned(unknown_contract, "Script is unknown"),
                Error::new(
                    abigen_command.span,
                    format!(
                        "Consider adding: Script(name=\"{}\", project=...)",
                        unknown_contract.value()
                    ),
                ),
            ]
        })
        .validate_no_errors()
}

pub(crate) fn validate_zero_or_one_wallet_command_present(
    commands: &[InitializeWalletCommand],
) -> Result<()> {
    if commands.len() > 1 {
        commands
            .iter()
            .map(|command| Error::new(command.span, "Only one `Wallets` command allowed"))
            .combine_errors()
            .map(Err)
            .expect("Known to have at least one error")
    } else {
        Ok(())
    }
}

fn names_of_program_bindings(
    commands: &AbigenCommand,
    program_type: ProgramType,
) -> HashSet<&LitStr> {
    commands
        .targets
        .iter()
        .filter_map(|target| (target.program_type == program_type).then_some(&target.name))
        .collect()
}

fn extract_contracts_to_deploy(commands: &[DeployContractCommand]) -> HashSet<&LitStr> {
    commands.iter().map(|c| &c.contract).collect()
}

fn extract_scripts_to_load(commands: &[LoadScriptCommand]) -> HashSet<&LitStr> {
    commands.iter().map(|c| &c.script).collect()
}
