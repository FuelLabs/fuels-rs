use crate::attributes::Attributes;
use fuels_core::code_gen::abigen::{AbigenTarget, ProgramType};
use itertools::Itertools;
use proc_macro2::{Ident, Span};
use std::collections::HashSet;
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input::ParseMacroInput,
    punctuated::Punctuated,
    {parenthesized, Error, LitStr, Result as ParseResult, Token},
};

impl From<MacroAbigenTargets> for Vec<AbigenTarget> {
    fn from(targets: MacroAbigenTargets) -> Self {
        targets.targets.into_iter().map(Into::into).collect()
    }
}

impl From<MacroAbigenTarget> for AbigenTarget {
    fn from(macro_target: MacroAbigenTarget) -> Self {
        AbigenTarget {
            name: macro_target.name,
            abi: macro_target.abi,
            program_type: macro_target.program_type,
        }
    }
}

// Although identical to `AbigenTarget` from fuels-core, due to the orphan rule
// we cannot implement Parse for the latter.
struct MacroAbigenTarget {
    name: String,
    abi: String,
    program_type: ProgramType,
}

pub(crate) struct MacroAbigenTargets {
    targets: Punctuated<MacroAbigenTarget, Token![,]>,
}

impl Parse for MacroAbigenTargets {
    fn parse(input: ParseStream) -> ParseResult<Self> {
        let abis = input.parse_terminated(ParseMacroInput::parse)?;

        Ok(Self { targets: abis })
    }
}

impl Parse for MacroAbigenTarget {
    fn parse(input: ParseStream) -> ParseResult<Self> {
        let program_type = Self::parse_program_type(input)?;

        let attrs = Attributes::new(input, &["name", "abi"])?;
        let name = attrs.get_as_str("name")?;
        let abi = attrs.get_as_str("abi")?;

        Ok(Self {
            name,
            abi,
            program_type,
        })
    }
}

impl MacroAbigenTarget {
    fn parse_program_type(input: ParseStream) -> ParseResult<ProgramType> {
        let ident = input.parse::<Ident>()?;

        match ident.to_string().as_ref() {
            "Contract" => Ok(ProgramType::Contract),
            "Script" => Ok(ProgramType::Script),
            "Predicate" => Ok(ProgramType::Predicate),
            _ => Err(Error::new_spanned(
                ident,
                "Unsupported program type. Expected: 'Contract', 'Script' or 'Predicate'",
            )),
        }
    }
}

pub(crate) enum Command {
    Wallets {
        span: Span,
        names: Vec<LitStr>,
    },
    Abigen {
        name: String,
        abi: String,
    },
    Deploy {
        name: String,
        contract: LitStr,
        wallet: String,
    },
}

impl Parse for Command {
    fn parse(input: ParseStream) -> ParseResult<Self> {
        let span = input.span();
        let ident = input.parse::<Ident>()?;

        match ident.to_string().as_ref() {
            "Wallets" => {
                let content;
                parenthesized!(content in input);

                let wallets = Punctuated::<LitStr, Token![,]>::parse_terminated(&content)?
                    .into_iter()
                    .collect();

                Ok(Command::Wallets {
                    span,
                    names: wallets,
                })
            }

            "Abigen" => {
                let attributes = Attributes::new(input, &["name", "abi"])?;
                let name = attributes.get_as_str("name")?;
                let abi = attributes.get_as_str("abi")?;

                Ok(Command::Abigen { name, abi })
            }
            "Deploy" => {
                let attributes = Attributes::new(input, &["name", "contract", "wallet"])?;
                let name = attributes.get_as_str("name")?;
                let contract = attributes.get_as_lit_str("contract")?;
                let wallet = attributes.get_as_str("wallet")?;

                Ok(Command::Deploy {
                    name,
                    contract,
                    wallet,
                })
            }
            _ => Err(Error::new_spanned(
                ident,
                "Unsupported command. Expected: 'Wallets', 'Abigen' or 'Deploy'",
            )),
        }
    }
}

pub(crate) struct TestContractCommands {
    pub(crate) commands: Vec<Command>,
}

impl Parse for TestContractCommands {
    fn parse(input: ParseStream) -> ParseResult<Self> {
        let span = input.span();
        let commands: Punctuated<Command, Token![,]> =
            input.parse_terminated(ParseMacroInput::parse)?;

        let contract_to_abigen: HashSet<_> = commands
            .iter()
            .filter_map(|c| {
                if let Command::Abigen { name, .. } = c {
                    return Some(name);
                }
                None
            })
            .collect();

        let maybe_err = commands
            .iter()
            .filter_map(|c| {
                if let Command::Deploy { contract, .. } = c {
                    let contract_name = contract.value();
                    if !contract_to_abigen.contains(&contract_name) {
                        let suggestion = syn::Error::new(
                            span,
                            format!("Consider adding: Abigen(name=\"{contract_name}\", abi=...)"),
                        );

                        let mut unknown_contract_err = syn::Error::new_spanned(
                            contract,
                            format!("{contract_name} is unknown"),
                        );
                        unknown_contract_err.combine(suggestion);

                        return Some(unknown_contract_err);
                    }
                }
                None
            })
            .reduce(|mut all_errors: syn::Error, error: syn::Error| {
                all_errors.combine(error);
                all_errors
            });

        if let Some(err) = maybe_err {
            return Err(err);
        }

        let wallet_commands: Vec<_> = commands
            .iter()
            .filter_map(|c| {
                if let Command::Wallets { span, .. } = c {
                    return Some(span);
                }
                None
            })
            .cloned()
            .collect();

        if wallet_commands.len() > 1 {
            let wallet_errors = wallet_commands
                .into_iter()
                .map(|s| syn::Error::new(s, "Only one `Wallets` command allowed"))
                .reduce(|mut all_errors, error| {
                    all_errors.combine(error);
                    all_errors
                })
                .expect("Should contain at least one element");

            return Err(wallet_errors);
        }

        let mayb_wallet_names_err = commands
            .iter()
            .filter_map(|c| {
                if let Command::Wallets { names, .. } = c {
                    return names
                        .iter()
                        .sorted_by_key(|n| n.value())
                        .group_by(|n| *n)
                        .into_iter()
                        .filter_map(|(_, group)| {
                            let group: Vec<_> = group.collect();
                            if group.len() > 1 {
                                return Some(
                                    group
                                        .into_iter()
                                        .map(|g| {
                                            syn::Error::new_spanned(g, "Dupplicate wallet entry")
                                        })
                                        .reduce(|mut all_errors, error| {
                                            all_errors.combine(error);
                                            all_errors
                                        })
                                        .unwrap(),
                                );
                            }
                            None
                        })
                        .reduce(|mut all_errors, error| {
                            all_errors.combine(error);
                            all_errors
                        });
                }
                None
            })
            .reduce(|mut all_errors, error| {
                all_errors.combine(error);
                all_errors
            });

        if let Some(err) = mayb_wallet_names_err {
            return Err(err);
        }

        Ok(Self {
            commands: commands.into_iter().collect(),
        })
    }
}
