use std::convert::TryFrom;

use fuels_code_gen::ProgramType;
use proc_macro2::Span;
use syn::{Error, LitStr};

use crate::{
    parse_utils::{Command, UniqueNameValues},
    setup_program_test::parsing::commands::MacroCommand,
};

#[derive(Debug)]
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

#[derive(Debug)]
pub(crate) struct AbigenCommand {
    pub(crate) span: Span,
    pub(crate) targets: Vec<TargetInfo>,
}

impl MacroCommand for AbigenCommand {
    fn expected_name() -> &'static str {
        "Abigen"
    }
}

impl TryFrom<Command> for AbigenCommand {
    type Error = Error;

    fn try_from(command: Command) -> Result<Self, Self::Error> {
        Self::validate_command_name(&command)?;

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
