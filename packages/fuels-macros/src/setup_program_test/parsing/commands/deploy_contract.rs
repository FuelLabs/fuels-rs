use std::convert::TryFrom;

use syn::{Error, Lit, LitStr};

use crate::parse_utils::{Command, UniqueNameValues};

#[derive(Debug, Clone)]
pub struct DeployContractCommand {
    pub name: String,
    pub contract: LitStr,
    pub wallet: String,
    pub random_salt: bool,
}

impl TryFrom<Command> for DeployContractCommand {
    type Error = Error;

    fn try_from(command: Command) -> Result<Self, Self::Error> {
        let name_values = UniqueNameValues::new(command.contents)?;
        name_values.validate_has_no_other_names(&["name", "contract", "wallet", "random_salt"])?;

        let name = name_values.get_as_lit_str("name")?.value();
        let contract = name_values.get_as_lit_str("contract")?.clone();
        let wallet = name_values.get_as_lit_str("wallet")?.value();
        let random_salt = name_values.try_get("random_salt").map_or(true, |opt| {
            let Lit::Bool(b) = opt else { return true };
            b.value()
        });

        Ok(Self {
            name,
            contract,
            wallet,
            random_salt,
        })
    }
}
