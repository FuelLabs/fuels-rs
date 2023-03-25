use std::collections::HashSet;

use fuels_code_gen::ProgramType;
use itertools::{chain, Itertools};
use proc_macro2::Span;
use syn::{
    parse::{Parse, ParseStream},
    Error, LitStr, Result as ParseResult,
};

use crate::{
    parse_utils::{Command, ErrorsExt},
    setup_program_test::parsing::commands::{
        AbigenCommand, DeployContract, InitializeWallet, LoadScript, MacroCommand,
    },
};

fn parse_test_program_commands(
    input: ParseStream,
) -> syn::Result<(
    Vec<InitializeWallet>,
    Vec<AbigenCommand>,
    Vec<DeployContract>,
    Vec<LoadScript>,
)> {
    let commands = Command::parse_multiple(input)?;

    let mut init_wallets: Vec<syn::Result<InitializeWallet>> = vec![];
    let mut gen_contracts: Vec<syn::Result<AbigenCommand>> = vec![];
    let mut deploy_contracts: Vec<syn::Result<DeployContract>> = vec![];
    let mut load_scripts: Vec<syn::Result<LoadScript>> = vec![];

    let mut errors = vec![];

    for command in commands {
        let command_name = &command.name;
        if command_name == InitializeWallet::expected_name() {
            init_wallets.push(command.try_into());
        } else if command_name == AbigenCommand::expected_name() {
            gen_contracts.push(command.try_into());
        } else if command_name == DeployContract::expected_name() {
            deploy_contracts.push(command.try_into());
        } else if command_name == LoadScript::expected_name() {
            load_scripts.push(command.try_into());
        } else {
            errors.push(Error::new_spanned(
                command.name,
                "Unsupported command. Expected: 'Wallets', 'Abigen', 'Deploy' or 'LoadScript'",
            ))
        }
    }

    let (init_wallets, wallet_errs): (Vec<_>, Vec<_>) = init_wallets.into_iter().partition_result();
    let (gen_contracts, gen_errs): (Vec<_>, Vec<_>) = gen_contracts.into_iter().partition_result();
    let (deploy_contracts, deploy_errs): (Vec<_>, Vec<_>) =
        deploy_contracts.into_iter().partition_result();
    let (load_scripts, load_script_errs): (Vec<_>, Vec<_>) =
        load_scripts.into_iter().partition_result();

    chain!(errors, wallet_errs, gen_errs, deploy_errs, load_script_errs).validate_no_errors()?;

    Ok((init_wallets, gen_contracts, deploy_contracts, load_scripts))
}

// Contains the result of parsing the input to the `setup_program_test` macro.
// Contents represent the users wishes with regards to wallet initialization,
// bindings generation and contract deployment.
pub(crate) struct TestProgramCommands {
    pub(crate) initialize_wallets: Option<InitializeWallet>,
    pub(crate) generate_bindings: AbigenCommand,
    pub(crate) deploy_contract: Vec<DeployContract>,
    pub(crate) load_scripts: Vec<LoadScript>,
}

impl Parse for TestProgramCommands {
    fn parse(input: ParseStream) -> ParseResult<Self> {
        let span = input.span();
        let (mut initialize_wallets, generate_contract, deploy_contract, load_scripts) =
            parse_test_program_commands(input)?;

        let abigen_command = extract_the_abigen_command(span, generate_contract)?;

        validate_all_contracts_are_known(&abigen_command, deploy_contract.as_slice())?;
        validate_all_scripts_are_known(&abigen_command, load_scripts.as_slice())?;

        validate_zero_or_one_wallet_command_present(initialize_wallets.as_slice())?;

        Ok(Self {
            initialize_wallets: initialize_wallets.pop(),
            generate_bindings: abigen_command,
            deploy_contract,
            load_scripts,
        })
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

fn contracts_to_deploy(commands: &[DeployContract]) -> HashSet<&LitStr> {
    commands.iter().map(|c| &c.contract).collect()
}

fn scripts_to_load(commands: &[LoadScript]) -> HashSet<&LitStr> {
    commands.iter().map(|c| &c.script).collect()
}

fn extract_the_abigen_command(
    parent_span: Span,
    mut commands: Vec<AbigenCommand>,
) -> Result<AbigenCommand, Error> {
    if commands.len() != 1 {
        let err = commands
            .iter()
            .map(|command| Error::new(command.span, "Only one `Abigen` command allowed"))
            .combine_errors()
            .unwrap_or_else(|| Error::new(parent_span, "Add an `Abigen(..)` command!"));

        Err(err)
    } else {
        Ok(commands.pop().unwrap())
    }
}

fn validate_all_contracts_are_known(
    abigen_command: &AbigenCommand,
    deploy_contracts: &[DeployContract],
) -> syn::Result<()> {
    contracts_to_deploy(deploy_contracts)
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
                        "Consider adding: Contract(name=\"{}\", abi=...)",
                        unknown_contract.value()
                    ),
                ),
            ]
        })
        .validate_no_errors()
}

fn validate_all_scripts_are_known(
    abigen_command: &AbigenCommand,
    load_scripts: &[LoadScript],
) -> syn::Result<()> {
    scripts_to_load(load_scripts)
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
                        "Consider adding: Script(name=\"{}\", abi=...)",
                        unknown_contract.value()
                    ),
                ),
            ]
        })
        .validate_no_errors()
}

fn validate_zero_or_one_wallet_command_present(commands: &[InitializeWallet]) -> syn::Result<()> {
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
