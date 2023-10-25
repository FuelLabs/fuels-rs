use std::convert::TryFrom;

use proc_macro2::Span;
use syn::Error;

use crate::parse_utils::Command;

#[derive(Debug, Clone)]
pub struct RunOnLiveNodeCommand {
    pub span: Span,
}

impl TryFrom<Command> for RunOnLiveNodeCommand {
    type Error = Error;

    fn try_from(command: Command) -> Result<Self, Self::Error> {
        if command.contents.is_empty() {
            Ok(Self {
                span: command.name.span(),
            })
        } else {
            Err(Error::new(
                command.name.span(),
                "`RunOnLiveNode` does not accept arguments",
            ))
        }
    }
}
