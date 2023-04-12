use std::convert::TryFrom;

use fuels_code_gen::ProgramType;
use proc_macro2::Span;
use syn::{Error, LitStr};

use crate::parse_utils::{Command, UniqueNameValues};

#[derive(Debug, Clone)]
pub(crate) struct TargetInfo {
    pub(crate) name: LitStr,
    pub(crate) project: LitStr,
    pub(crate) program_type: ProgramType,
}

impl TryFrom<Command> for TargetInfo {
    type Error = Error;

    fn try_from(command: Command) -> Result<Self, Self::Error> {
        let program_type = command.name.try_into()?;

        let name_values = UniqueNameValues::new(command.contents)?;
        name_values.validate_has_no_other_names(&["name", "project"])?;

        let name = name_values.get_as_lit_str("name")?.clone();
        let project = name_values.get_as_lit_str("project")?.clone();

        Ok(Self {
            name,
            project,
            program_type,
        })
    }
}

#[derive(Debug, Clone)]
pub(crate) struct AbigenCommand {
    pub(crate) span: Span,
    pub(crate) targets: Vec<TargetInfo>,
}

impl TryFrom<Command> for AbigenCommand {
    type Error = Error;

    fn try_from(command: Command) -> Result<Self, Self::Error> {
        let targets = command
            .contents
            .into_iter()
            .map(|meta| Command::new(meta).and_then(TargetInfo::try_from))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self {
            span: command.name.span(),
            targets,
        })
    }
}
