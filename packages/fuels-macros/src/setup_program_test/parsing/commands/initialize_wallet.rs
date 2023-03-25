use std::convert::TryFrom;

use proc_macro2::Span;
use syn::{Error, LitStr};

use crate::{
    parse_utils::{Command, UniqueLitStrs},
    setup_program_test::parsing::commands::MacroCommand,
};

pub struct InitializeWallet {
    pub span: Span,
    pub names: Vec<LitStr>,
}

impl MacroCommand for InitializeWallet {
    fn expected_name() -> &'static str {
        "Wallets"
    }
}

impl TryFrom<Command> for InitializeWallet {
    type Error = Error;

    fn try_from(command: Command) -> Result<Self, Self::Error> {
        Self::validate_command_name(&command)?;

        let wallets = UniqueLitStrs::new(command.contents)?;

        Ok(Self {
            span: command.name.span(),
            names: wallets.into_iter().collect(),
        })
    }
}
