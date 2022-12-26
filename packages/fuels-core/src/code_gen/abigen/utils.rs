use quote::quote;

use fuels_types::errors::Error;

use crate::code_gen::abi_types::{FullABIFunction, FullProgramABI};
use crate::code_gen::generated_code::GeneratedCode;
use crate::source::Source;

pub struct AbigenTarget {
    pub name: String,
    pub source: String,
    pub program_type: ProgramType,
}

pub(crate) struct ParsedAbigenTarget {
    pub name: String,
    pub source: FullProgramABI,
    pub program_type: ProgramType,
}

impl TryFrom<AbigenTarget> for ParsedAbigenTarget {
    type Error = Error;

    fn try_from(value: AbigenTarget) -> Result<Self, Self::Error> {
        Ok(Self {
            name: value.name,
            source: parse_program_abi(&value.source)?,
            program_type: value.program_type,
        })
    }
}

pub(crate) fn limited_std_prelude() -> GeneratedCode {
    let code = quote! {
            use ::std::{
                clone::Clone,
                convert::{Into, TryFrom, From},
                format,
                iter::IntoIterator,
                iter::Iterator,
                marker::Sized,
                panic, vec,
                string::ToString
            };
    };

    GeneratedCode {
        code,
        ..Default::default()
    }
}

pub(crate) fn extract_main_fn(abi: &[FullABIFunction]) -> Result<&FullABIFunction, Error> {
    let candidates = abi
        .iter()
        .filter(|function| function.name() == "main")
        .collect::<Vec<_>>();

    match candidates.as_slice() {
        [single_main_fn] => Ok(single_main_fn),
        _ => {
            let fn_names = candidates
                .iter()
                .map(|candidate| candidate.name())
                .collect::<Vec<_>>();
            Err(Error::CompilationError(format!(
                "ABI must have one and only one function with the name 'main'. Got: {fn_names:?}"
            )))
        }
    }
}

fn parse_program_abi(abi_source: &str) -> Result<FullProgramABI, Error> {
    let source = Source::parse(abi_source).expect("failed to parse JSON ABI");
    let json_abi_str = source.get().expect("failed to parse JSON ABI from string");
    FullProgramABI::from_json_abi(&json_abi_str)
}

#[derive(Clone, Copy)]
pub enum ProgramType {
    Script,
    Contract,
    Predicate,
}
