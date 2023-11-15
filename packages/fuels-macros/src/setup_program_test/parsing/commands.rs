pub(crate) use abigen::AbigenCommand;
pub(crate) use deploy_contract::DeployContractCommand;
pub(crate) use initialize_wallet::InitializeWalletCommand;
use itertools::Itertools;
pub(crate) use load_script::LoadScriptCommand;
pub(crate) use run_on_live_node::RunOnLiveNodeCommand;
use syn::{
    parse::{Parse, ParseStream},
    Result,
};

use crate::setup_program_test::parsing::{
    command_parser::command_parser,
    validations::{
        extract_the_abigen_command, validate_all_contracts_are_known,
        validate_all_scripts_are_known, validate_zero_or_one_wallet_command_present,
    },
};

use super::validations::should_run_on_live_node_command;

mod abigen;
mod deploy_contract;
mod initialize_wallet;
mod load_script;
mod run_on_live_node;

// Contains the result of parsing the input to the `setup_program_test` macro.
// Contents represent the users wishes with regards to wallet initialization,
// bindings generation and contract deployment.
pub(crate) struct TestProgramCommands {
    pub(crate) initialize_wallets: Option<InitializeWalletCommand>,
    pub(crate) generate_bindings: AbigenCommand,
    pub(crate) deploy_contract: Vec<DeployContractCommand>,
    pub(crate) load_scripts: Vec<LoadScriptCommand>,
    pub(crate) run_on_live_node: bool,
}

command_parser!(
    Wallets -> InitializeWalletCommand,
    Abigen -> AbigenCommand,
    Deploy -> DeployContractCommand,
    LoadScript -> LoadScriptCommand,
    RunOnLiveNode -> RunOnLiveNodeCommand,
);

impl Parse for TestProgramCommands {
    fn parse(input: ParseStream) -> Result<Self> {
        let span = input.span();

        let mut parsed_commands = CommandParser::parse(input)?;

        let abigen_command = extract_the_abigen_command(span, &parsed_commands.Abigen)?;

        validate_all_contracts_are_known(&abigen_command, &parsed_commands.Deploy)?;

        validate_all_scripts_are_known(&abigen_command, &parsed_commands.LoadScript)?;

        validate_zero_or_one_wallet_command_present(&parsed_commands.Wallets)?;

        let run_on_live_node = should_run_on_live_node_command(&parsed_commands.RunOnLiveNode)?;

        Ok(Self {
            initialize_wallets: parsed_commands.Wallets.pop(),
            generate_bindings: abigen_command,
            deploy_contract: parsed_commands.Deploy,
            load_scripts: parsed_commands.LoadScript,
            run_on_live_node,
        })
    }
}
