use std::collections::HashSet;

use itertools::{chain, Itertools};
use proc_macro2::Span;
use syn::{
    parse::{Parse, ParseStream},
    Error, LitStr, Result as ParseResult,
};

use crate::parse_utils::{combine_errors, Command, UniqueLitStrs, UniqueNameValues};

trait MacroName {
    fn macro_command_name() -> &'static str;
}

pub(crate) struct InitializeWallet {
    pub(crate) span: Span,
    pub(crate) names: Vec<LitStr>,
}

impl MacroName for InitializeWallet {
    fn macro_command_name() -> &'static str {
        "Wallets"
    }
}

impl TryFrom<Command> for InitializeWallet {
    type Error = Error;

    fn try_from(command: Command) -> Result<Self, Self::Error> {
        validate_command_has_correct_name::<Self>(&command)?;

        let wallets = UniqueLitStrs::new(command.contents)?;

        Ok(Self {
            span: command.name.span(),
            names: wallets.into_iter().collect(),
        })
    }
}

pub(crate) struct GenerateContract {
    pub(crate) name: LitStr,
    pub(crate) abi: LitStr,
}

impl MacroName for GenerateContract {
    fn macro_command_name() -> &'static str {
        "Abigen"
    }
}

impl TryFrom<Command> for GenerateContract {
    type Error = Error;

    fn try_from(command: Command) -> Result<Self, Self::Error> {
        validate_command_has_correct_name::<Self>(&command)?;

        let name_values = UniqueNameValues::new(command.contents)?;
        name_values.validate_has_no_other_names(&["name", "abi"])?;

        let name = name_values.get_as_lit_str("name")?.clone();
        let abi = name_values.get_as_lit_str("abi")?.clone();

        Ok(Self { name, abi })
    }
}

pub(crate) struct DeployContract {
    pub name: String,
    pub contract: LitStr,
    pub wallet: String,
}

impl MacroName for DeployContract {
    fn macro_command_name() -> &'static str {
        "Deploy"
    }
}

impl TryFrom<Command> for DeployContract {
    type Error = Error;

    fn try_from(command: Command) -> Result<Self, Self::Error> {
        validate_command_has_correct_name::<Self>(&command)?;
        let name_values = UniqueNameValues::new(command.contents)?;
        name_values.validate_has_no_other_names(&["name", "contract", "wallet"])?;

        let name = name_values.get_as_lit_str("name")?.value();
        let contract = name_values.get_as_lit_str("contract")?.clone();
        let wallet = name_values.get_as_lit_str("wallet")?.value();

        Ok(Self {
            name,
            contract,
            wallet,
        })
    }
}

fn validate_command_has_correct_name<T: MacroName>(command: &Command) -> syn::Result<()> {
    let expected_name = T::macro_command_name();
    if command.name == expected_name {
        Ok(())
    } else {
        Err(Error::new_spanned(
            command.name.clone(),
            format!("Expected command to have name: '{expected_name}'."),
        ))
    }
}

fn parse_test_contract_commands(
    input: ParseStream,
) -> syn::Result<(
    Vec<InitializeWallet>,
    Vec<GenerateContract>,
    Vec<DeployContract>,
)> {
    let commands = Command::parse_multiple(input)?;

    let mut init_wallets: Vec<syn::Result<InitializeWallet>> = vec![];
    let mut gen_contracts: Vec<syn::Result<GenerateContract>> = vec![];
    let mut deploy_contracts: Vec<syn::Result<DeployContract>> = vec![];

    let mut errors = vec![];

    for command in commands {
        let command_name = &command.name;
        if command_name == InitializeWallet::macro_command_name() {
            init_wallets.push(command.try_into());
        } else if command_name == GenerateContract::macro_command_name() {
            gen_contracts.push(command.try_into());
        } else if command_name == DeployContract::macro_command_name() {
            deploy_contracts.push(command.try_into());
        } else {
            errors.push(Error::new_spanned(
                command.name,
                "Unsupported command. Expected: 'Wallets', 'Abigen' or 'Deploy'",
            ))
        }
    }

    let (init_wallets, wallet_errs): (Vec<_>, Vec<_>) = init_wallets.into_iter().partition_result();
    let (gen_contracts, gen_errs): (Vec<_>, Vec<_>) = gen_contracts.into_iter().partition_result();
    let (deploy_contracts, deploy_errs): (Vec<_>, Vec<_>) =
        deploy_contracts.into_iter().partition_result();

    if let Some(err) = combine_errors(chain!(errors, wallet_errs, gen_errs, deploy_errs)) {
        Err(err)
    } else {
        Ok((init_wallets, gen_contracts, deploy_contracts))
    }
}

pub(crate) struct TestContractCommands {
    pub(crate) initialize_wallets: Option<InitializeWallet>,
    pub(crate) generate_contract: Vec<GenerateContract>,
    pub(crate) deploy_contract: Vec<DeployContract>,
}

impl Parse for TestContractCommands {
    fn parse(input: ParseStream) -> ParseResult<Self> {
        let span = input.span();

        let (mut initialize_wallets, generate_contract, deploy_contract) =
            parse_test_contract_commands(input)?;

        Self::validate_all_contracts_are_known(
            span,
            generate_contract.as_slice(),
            deploy_contract.as_slice(),
        )?;

        Self::validate_zero_or_one_wallet_command_present(initialize_wallets.as_slice())?;

        Ok(Self {
            initialize_wallets: initialize_wallets.pop(),
            generate_contract,
            deploy_contract,
        })
    }
}

impl TestContractCommands {
    fn names_of_generated_contracts(commands: &[GenerateContract]) -> HashSet<&LitStr> {
        commands.iter().map(|c| &c.name).collect()
    }

    fn names_of_deployed_contracts(commands: &[DeployContract]) -> HashSet<&LitStr> {
        commands.iter().map(|c| &c.contract).collect()
    }

    fn validate_all_contracts_are_known(
        span: Span,
        generate_contracts: &[GenerateContract],
        deploy_contracts: &[DeployContract],
    ) -> syn::Result<()> {
        let map = Self::names_of_deployed_contracts(deploy_contracts)
            .difference(&Self::names_of_generated_contracts(generate_contracts))
            .map(|unknown_contract| {
                let mut unknown_contract_err =
                    Error::new_spanned(unknown_contract, "Contract is unknown");

                unknown_contract_err.combine(Error::new(
                    span,
                    format!(
                        "Consider adding: Abigen(name=\"{}\", abi=...)",
                        unknown_contract.value()
                    ),
                ));

                unknown_contract_err
            })
            .collect::<Vec<_>>();

        let maybe_missing_contracts = combine_errors(map);

        if let Some(err) = maybe_missing_contracts {
            Err(err)
        } else {
            Ok(())
        }
    }

    fn validate_zero_or_one_wallet_command_present(
        commands: &[InitializeWallet],
    ) -> syn::Result<()> {
        if commands.len() > 1 {
            combine_errors(
                commands
                    .iter()
                    .map(|command| Error::new(command.span, "Only one `Wallets` command allowed"))
                    .collect::<Vec<_>>(),
            )
            .map(Err)
            .expect("Known to have at least one error")
        } else {
            Ok(())
        }
    }
}
