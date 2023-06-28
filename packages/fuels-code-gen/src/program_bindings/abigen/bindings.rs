use crate::{
    error::Result,
    program_bindings::{
        abigen::{
            abigen_target::ParsedAbigenTarget,
            bindings::{
                contract::contract_bindings, predicate::predicate_bindings, script::script_bindings,
            },
            ProgramType,
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

pub(crate) fn generate_bindings(target: ParsedAbigenTarget, no_std: bool) -> Result<GeneratedCode> {
    let bindings_generator = match target.program_type {
        ProgramType::Script => script_bindings,
        ProgramType::Contract => contract_bindings,
        ProgramType::Predicate => predicate_bindings,
    };

    let name = ident(&target.name);
    let abi = target.source;
    bindings_generator(&name, abi, no_std)
}
