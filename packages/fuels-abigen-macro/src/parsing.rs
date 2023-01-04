use proc_macro2::Ident;
use syn::parse::{Parse, ParseStream};
use syn::parse_macro_input::ParseMacroInput;
use syn::punctuated::Punctuated;
use syn::{Error, LitStr, Result as ParseResult, Token};

use fuels_core::code_gen::abigen::{AbigenTarget, ProgramType};

use crate::attributes::Attributes;

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
    targets: Punctuated<MacroAbigenTarget, Token![,]>,
}

impl Parse for MacroAbigenTargets {
    fn parse(input: ParseStream) -> ParseResult<Self> {
        let abis = input.parse_terminated(ParseMacroInput::parse)?;

        Ok(Self { targets: abis })
    }
}

impl Parse for MacroAbigenTarget {
    fn parse(input: ParseStream) -> ParseResult<Self> {
        let program_type = Self::parse_program_type(input)?;

        let attrs: Attributes = Parse::parse(input)?;

        let name = attrs.get_as_str("name")?;
        let abi = attrs.get_as_str("abi")?;

        Ok(Self {
            name,
            abi,
            program_type,
        })
    }
}

impl MacroAbigenTarget {
    fn parse_program_type(input: ParseStream) -> ParseResult<ProgramType> {
        let ident = input.parse::<Ident>()?;

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

/// Contract procedural macro arguments.
pub(crate) struct ContractTestArgs {
    pub(crate) instance_name: String,
    pub(crate) wallet_name: String,
    pub(crate) project_path: String,
}

impl Parse for ContractTestArgs {
    fn parse(input: ParseStream) -> ParseResult<Self> {
        let instance_name = input.parse::<Ident>()?.to_string();
        input.parse::<Token![,]>()?;

        let wallet_name = input.parse::<Ident>()?.to_string();
        input.parse::<Token![,]>()?;

        let project_path = input.parse::<LitStr>()?.value();
        if !input.is_empty() {
            input.parse::<Token![,]>()?;
        }

        Ok(ContractTestArgs {
            instance_name,
            wallet_name,
            project_path,
        })
    }
}
