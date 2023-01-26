use proc_macro2::Ident;
use syn::{
    parse::{Parse, ParseStream},
    Error, Result as ParseResult,
};

use crate::{
    abigen::{AbigenTarget, ProgramType},
    parse_utils::{Command, UniqueNameValues},
};

impl From<MacroAbigenTargets> for Vec<AbigenTarget> {
    fn from(targets: MacroAbigenTargets) -> Self {
        targets.targets.into_iter().map(Into::into).collect()
    }
}

impl From<MacroAbigenTarget> for AbigenTarget {
    fn from(macro_target: MacroAbigenTarget) -> Self {
        AbigenTarget {
            name: macro_target.name,
            abi: macro_target.abi,
            program_type: macro_target.program_type,
        }
    }
}

// Although identical to `AbigenTarget` from fuels-core, due to the orphan rule
// we cannot implement Parse for the latter.
struct MacroAbigenTarget {
    name: String,
    abi: String,
    program_type: ProgramType,
}

pub(crate) struct MacroAbigenTargets {
    targets: Vec<MacroAbigenTarget>,
}

impl Parse for MacroAbigenTargets {
    fn parse(input: ParseStream) -> ParseResult<Self> {
        let targets = Command::parse_multiple(input)?
            .into_iter()
            .map(MacroAbigenTarget::new)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self { targets })
    }
}

impl MacroAbigenTarget {
    pub fn new(command: Command) -> syn::Result<Self> {
        let program_type = Self::parse_program_type(command.name)?;

        let name_values = UniqueNameValues::new(command.contents)?;
        name_values.validate_has_no_other_names(&["name", "abi"])?;

        let name = name_values.get_as_lit_str("name")?.value();
        let abi = name_values.get_as_lit_str("abi")?.value();

        Ok(Self {
            name,
            abi,
            program_type,
        })
    }

    fn parse_program_type(ident: Ident) -> ParseResult<ProgramType> {
        match ident.to_string().as_ref() {
            "Contract" => Ok(ProgramType::Contract),
            "Script" => Ok(ProgramType::Script),
            "Predicate" => Ok(ProgramType::Predicate),
            _ => Err(Error::new_spanned(
                ident,
                "Unsupported program type. Expected: 'Contract', 'Script' or 'Predicate'",
            )),
        }
    }
}
