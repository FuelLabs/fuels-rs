use std::collections::{HashMap, HashSet};

use crate::experimental;
use crate::experimental::{
    combine_errors, parse_commands, Command, UniqueLitStrs, UniqueNameValues,
};
use itertools::{chain, Itertools};
use proc_macro2::{Ident, Span};
use syn::{
    parenthesized,
    parse::{Parse, ParseStream},
    parse_macro_input::ParseMacroInput,
    punctuated::Punctuated,
    Error, LitStr, Result as ParseResult, Token,
};

pub(crate) struct InitializeWallet {
    pub(crate) span: Span,
    pub(crate) names: Vec<LitStr>,
}

impl TryFrom<Command> for InitializeWallet {
    type Error = Error;

    fn try_from(command: Command) -> Result<Self, Self::Error> {
        let wallets = UniqueLitStrs::new(command.contents)?;

        Ok(Self {
            span: wallets.span(),
            names: wallets.into_iter().collect(),
        })
    }
}

pub(crate) struct GenerateContract {
    pub(crate) name: LitStr,
    pub(crate) abi: String,
}

impl TryFrom<Command> for GenerateContract {
    type Error = syn::Error;

    fn try_from(command: Command) -> Result<Self, Self::Error> {
        let name_values = UniqueNameValues::new(command.contents)?;
        name_values.validate_has_no_other_names(&["name", "abi"])?;

        let name = name_values.get_as_lit_str("name")?.clone();
        let abi = name_values.get_as_lit_str("abi")?.value();

        Ok(Self { name, abi })
    }
}

pub(crate) struct DeployContract {
    pub name: String,
    pub contract: LitStr,
    pub wallet: String,
}

impl TryFrom<Command> for DeployContract {
    type Error = syn::Error;

    fn try_from(command: Command) -> Result<Self, Self::Error> {
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

fn parse_test_contract_commands(
    input: ParseStream,
) -> syn::Result<(
    Vec<InitializeWallet>,
    Vec<GenerateContract>,
    Vec<DeployContract>,
)> {
    let commands = parse_commands(input)?;

    let mut init_wallets: Vec<syn::Result<InitializeWallet>> = vec![];
    let mut gen_contracts: Vec<syn::Result<GenerateContract>> = vec![];
    let mut deploy_contracts: Vec<syn::Result<DeployContract>> = vec![];

    let mut errors = vec![];

    for command in commands {
        match command.name.to_string().as_ref() {
            "Wallets" => init_wallets.push(command.try_into()),

            "Abigen" => gen_contracts.push(command.try_into()),
            "Deploy" => deploy_contracts.push(command.try_into()),
            _ => errors.push(Error::new_spanned(
                command.name,
                "Unsupported command. Expected: 'Wallets', 'Abigen' or 'Deploy'",
            )),
        }
    }

    let (a, err_a): (Vec<_>, Vec<_>) = init_wallets.into_iter().partition_result();
    let (b, err_b): (Vec<_>, Vec<_>) = gen_contracts.into_iter().partition_result();
    let (c, err_c): (Vec<_>, Vec<_>) = deploy_contracts.into_iter().partition_result();

    if let Some(err) = combine_errors(chain!(errors, err_a, err_b, err_c)) {
        Err(err)
    } else {
        Ok((a, b, c))
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

        let maybe_missing_contracts = experimental::combine_errors(map);

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
                    .into_iter()
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
