pub(crate) use abigen::AbigenCommand;
pub(crate) use deploy_contract::DeployContract;
use fuels_code_gen::ProgramType;
pub(crate) use initialize_wallet::InitializeWallet;
pub(crate) use load_script::LoadScript;
use proc_macro2::Span;
use syn::{Error, LitStr};

use crate::parse_utils::Command;

mod abigen;
mod deploy_contract;
mod initialize_wallet;
mod load_script;

pub(crate) trait MacroCommand: TryFrom<Command> {
    fn expected_name() -> &'static str;
    fn validate_command_name(command: &Command) -> syn::Result<()> {
        let expected_name = Self::expected_name();
        if command.name == expected_name {
            Ok(())
        } else {
            Err(Error::new_spanned(
                command.name.clone(),
                format!("Expected command to have name: '{expected_name}'."),
            ))
        }
    }
}
