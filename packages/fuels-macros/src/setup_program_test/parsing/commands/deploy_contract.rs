use std::convert::TryFrom;

use syn::{Error, LitStr};

use crate::parse_utils::{Command, UniqueNameValues};

#[derive(Debug, Clone)]
pub struct DeployContractCommand {
    pub name: String,
    pub contract: LitStr,
    pub wallet: String,
}

impl TryFrom<Command> for DeployContractCommand {
    type Error = Error;

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
