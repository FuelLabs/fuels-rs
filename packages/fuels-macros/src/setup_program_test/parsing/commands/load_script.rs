use std::convert::TryFrom;

use syn::{Error, LitStr};

use crate::{
    parse_utils::{Command, UniqueNameValues},
    setup_program_test::parsing::commands::MacroCommand,
};

pub struct LoadScript {
    pub name: String,
    pub script: LitStr,
    pub wallet: String,
}

impl MacroCommand for LoadScript {
    fn expected_name() -> &'static str {
        "LoadScript"
    }
}

impl TryFrom<Command> for LoadScript {
    type Error = Error;

    fn try_from(command: Command) -> Result<Self, Self::Error> {
        Self::validate_command_name(&command)?;
        let name_values = UniqueNameValues::new(command.contents)?;
        name_values.validate_has_no_other_names(&["name", "script", "wallet"])?;

        let name = name_values.get_as_lit_str("name")?.value();
        let script = name_values.get_as_lit_str("script")?.clone();
        let wallet = name_values.get_as_lit_str("wallet")?.value();

        Ok(Self {
            name,
            script,
            wallet,
        })
    }
}
