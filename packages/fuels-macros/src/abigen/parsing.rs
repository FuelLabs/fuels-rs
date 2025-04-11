use fuels_code_gen::{Abi, AbigenTarget, ProgramType};
use syn::{
    LitStr, Result,
    parse::{Parse, ParseStream},
};

use crate::parse_utils::{Command, UniqueNameValues};

impl From<MacroAbigenTargets> for Vec<AbigenTarget> {
    fn from(targets: MacroAbigenTargets) -> Self {
        targets.targets.into_iter().map(Into::into).collect()
    }
}

impl From<MacroAbigenTarget> for AbigenTarget {
    fn from(macro_target: MacroAbigenTarget) -> Self {
        AbigenTarget::new(
            macro_target.name,
            macro_target.source,
            macro_target.program_type,
        )
    }
}

// Although identical to `AbigenTarget` from fuels-core, due to the orphan rule
// we cannot implement Parse for the latter.
#[derive(Debug)]
pub(crate) struct MacroAbigenTarget {
    pub(crate) name: String,
    pub(crate) source: Abi,
    pub program_type: ProgramType,
}

pub(crate) struct MacroAbigenTargets {
    targets: Vec<MacroAbigenTarget>,
}

impl Parse for MacroAbigenTargets {
    fn parse(input: ParseStream) -> Result<Self> {
        let targets = Command::parse_multiple(input)?
            .into_iter()
            .map(MacroAbigenTarget::new)
            .collect::<Result<_>>()?;

        Ok(Self { targets })
    }
}

impl MacroAbigenTarget {
    pub fn new(command: Command) -> Result<Self> {
        let program_type = command.name.try_into()?;

        let name_values = UniqueNameValues::new(command.contents)?;
        name_values.validate_has_no_other_names(&["name", "abi"])?;

        let name = name_values.get_as_lit_str("name")?.value();
        let abi_lit_str = name_values.get_as_lit_str("abi")?;
        let source = Self::parse_inline_or_load_abi(abi_lit_str)?;

        Ok(Self {
            name,
            source,
            program_type,
        })
    }

    fn parse_inline_or_load_abi(abi_lit_str: &LitStr) -> Result<Abi> {
        let abi_string = abi_lit_str.value();
        let abi_str = abi_string.trim();

        if abi_str.starts_with('{') || abi_str.starts_with('[') || abi_str.starts_with('\n') {
            abi_str.parse()
        } else {
            Abi::load_from(abi_str)
        }
        .map_err(|e| syn::Error::new(abi_lit_str.span(), e.to_string()))
    }
}
