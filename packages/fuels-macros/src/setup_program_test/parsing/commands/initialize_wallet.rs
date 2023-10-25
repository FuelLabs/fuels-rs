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
        let wallets = UniqueLitStrs::new(command.contents.clone())?;
        let names: Vec<LitStr> = wallets.into_iter().collect();

        if names.is_empty() {
            return Err(Error::new(
                command.name.span(),
                "`Wallets` command can not be empty",
            ));
        }

        Ok(Self {
            span: command.name.span(),
            names,
        })
    }
}
