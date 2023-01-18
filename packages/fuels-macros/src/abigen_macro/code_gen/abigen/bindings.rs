use std::collections::HashSet;

use fuels_types::errors::Error;

use crate::{
    code_gen::{
        abi_types::FullTypeDeclaration,
        abigen::{
            abigen_target::{ParsedAbigenTarget, ProgramType},
            bindings::{
                contract::contract_bindings, predicate::predicate_bindings, script::script_bindings,
            },
        },
        generated_code::GeneratedCode,
    },
    utils::ident,
};

mod contract;
mod function_generator;
mod predicate;
mod script;
mod utils;

pub(crate) fn generate_bindings(
    target: ParsedAbigenTarget,
    no_std: bool,
    shared_types: &HashSet<FullTypeDeclaration>,
) -> Result<GeneratedCode, Error> {
    let bindings_generator = match target.program_type {
        ProgramType::Script => script_bindings,
        ProgramType::Contract => contract_bindings,
        ProgramType::Predicate => predicate_bindings,
    };

    let name = ident(&target.name);
    let abi = target.source;
    bindings_generator(&name, abi, no_std, shared_types)
}
