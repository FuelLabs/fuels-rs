use std::{convert::TryFrom, path::PathBuf, str::FromStr};

use fuel_abi_types::abi::full_program::FullProgramABI;
use proc_macro2::Ident;

use crate::{error, error::Error};

#[derive(Debug, Clone)]
pub struct AbigenTarget {
    pub name: String,
    pub source: Abi,
    pub program_type: ProgramType,
}

#[derive(Debug, Clone)]
pub struct Abi {
    pub path: Option<PathBuf>,
    pub abi: FullProgramABI,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgramType {
    Script,
    Contract,
    Predicate,
}

impl FromStr for ProgramType {
    type Err = Error;

    fn from_str(string: &str) -> std::result::Result<Self, Self::Err> {
        let program_type = match string {
            "Script" => ProgramType::Script,
            "Contract" => ProgramType::Contract,
            "Predicate" => ProgramType::Predicate,
            _ => {
                return Err(error!(
                    "`{string}` is not a valid program type. Expected one of: `Script`, `Contract`, `Predicate`"
                ))
            }
        };

        Ok(program_type)
    }
}

impl TryFrom<Ident> for ProgramType {
    type Error = syn::Error;

    fn try_from(ident: Ident) -> std::result::Result<Self, Self::Error> {
        ident
            .to_string()
            .as_str()
            .parse()
            .map_err(|e| Self::Error::new(ident.span(), e))
    }
}
