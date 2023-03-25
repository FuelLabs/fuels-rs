use std::convert::TryFrom;

use syn::{Error, LitStr};

use crate::{
    parse_utils::{Command, UniqueNameValues},
    setup_program_test::parsing::commands::MacroCommand,
};

pub struct DeployContract {
    pub name: String,
    pub contract: LitStr,
    pub wallet: String,
}

impl MacroCommand for DeployContract {
    fn expected_name() -> &'static str {
        "Deploy"
    }
}

impl TryFrom<Command> for DeployContract {
    type Error = Error;

    fn try_from(command: Command) -> Result<Self, Self::Error> {
        Self::validate_command_name(&command)?;
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
