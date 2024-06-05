use std::{convert::TryFrom, fmt, str::FromStr};

use syn::Error;

use crate::parse_utils::{Command, UniqueNameValues};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum BuildProfile {
    Debug,
    #[default]
    Release,
}

impl FromStr for BuildProfile {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "debug" => Ok(Self::Debug),
            "release" => Ok(Self::Release),
            _ => Err(r#"invalid build profile option: must be "debug" or "release""#),
        }
    }
}

impl fmt::Display for BuildProfile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                BuildProfile::Debug => "debug",
                BuildProfile::Release => "release",
            }
        )
    }
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
