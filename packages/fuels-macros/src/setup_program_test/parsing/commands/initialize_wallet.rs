use std::convert::TryFrom;

use proc_macro2::Span;
use syn::{Error, LitStr};

use crate::parse_utils::{Command, UniqueLitStrs};

#[derive(Debug, Clone)]
pub struct InitializeWalletCommand {
    pub span: Span,
    pub names: Vec<LitStr>,
}

impl TryFrom<Command> for InitializeWalletCommand {
    type Error = Error;

    fn try_from(command: Command) -> Result<Self, Self::Error> {
        let wallets = UniqueLitStrs::new(command.contents)?;

        Ok(Self {
            span: command.name.span(),
            names: wallets.into_iter().collect(),
        })
    }
}
