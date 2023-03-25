use std::convert::TryFrom;

use itertools::Itertools;
use proc_macro2::Ident;
use strum::VariantNames;
use strum_macros::{EnumString, EnumVariantNames};

use crate::{
    error::{Error, Result},
    program_bindings::abi_types::FullProgramABI,
    utils::Source,
};

#[derive(Debug, Clone)]
pub struct AbigenTarget {
    pub name: String,
    pub abi: String,
    pub program_type: ProgramType,
}

pub(crate) struct ParsedAbigenTarget {
    pub name: String,
    pub source: FullProgramABI,
    pub program_type: ProgramType,
}

impl TryFrom<AbigenTarget> for ParsedAbigenTarget {
    type Error = Error;

    fn try_from(value: AbigenTarget) -> Result<Self> {
        Ok(Self {
            name: value.name,
            source: parse_program_abi(&value.abi)?,
            program_type: value.program_type,
        })
    }
}

fn parse_program_abi(abi_source: &str) -> Result<FullProgramABI> {
    let source = Source::parse(abi_source).expect("failed to parse JSON ABI");
    let json_abi_str = source.get().expect("failed to parse JSON ABI from string");
    FullProgramABI::from_json_abi(&json_abi_str)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumString, EnumVariantNames)]
pub enum ProgramType {
    Script,
    Contract,
    Predicate,
}

impl TryFrom<Ident> for ProgramType {
    type Error = syn::Error;

    fn try_from(ident: Ident) -> std::result::Result<Self, Self::Error> {
        ident.to_string().as_str().parse().map_err(|_| {
            let valid_program_types = ProgramType::VARIANTS
                .iter()
                .map(|variant| format!("'{variant}'"))
                .join(", ");
            Self::Error::new(
                ident.span(),
                format!("Not a valid program type. Expected one of: {valid_program_types}."),
            )
        })
    }
}
