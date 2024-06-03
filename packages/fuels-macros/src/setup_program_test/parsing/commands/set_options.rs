use std::convert::TryFrom;

use strum::{Display, EnumString};
use syn::Error;

use crate::parse_utils::{Command, UniqueNameValues};

#[derive(Debug, Clone, Default, PartialEq, Eq, EnumString, Display)]
#[strum(serialize_all = "lowercase")]
pub enum BuildProfile {
    Debug,
    #[default]
    Release,
}

#[derive(Debug, Clone, Default)]
pub struct SetOptionsCommand {
    pub profile: BuildProfile,
}

impl TryFrom<Command> for SetOptionsCommand {
    type Error = Error;

    fn try_from(command: Command) -> Result<Self, Self::Error> {
        let name_values = UniqueNameValues::new(command.contents)?;
        name_values.validate_has_no_other_names(&["profile"])?;

        let profile = name_values.get_as_lit_str("profile")?;
        let profile = profile
            .value()
            .as_str()
            .parse()
            .map_err(|msg| Error::new(profile.span(), msg))?;

        Ok(Self { profile })
    }
}
