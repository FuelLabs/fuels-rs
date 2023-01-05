use fuels_core::code_gen::abigen::Abigen;
use parsing::{MacroAbigenTargets, TestContractCommands};
use proc_macro::TokenStream;
use setup_contract_test::generate_setup_contract_test_code;
use syn::parse_macro_input;

mod attributes;
mod parsing;
mod setup_contract_test;

#[proc_macro]
pub fn abigen(input: TokenStream) -> TokenStream {
    let targets = parse_macro_input!(input as MacroAbigenTargets);

    Abigen::generate(targets.into(), false).unwrap().into()
}

#[proc_macro]
pub fn wasm_abigen(input: TokenStream) -> TokenStream {
    let targets = parse_macro_input!(input as MacroAbigenTargets);

    Abigen::generate(targets.into(), true).unwrap().into()
}

/// This proc macro is used to reduce the amount of boilerplate code in integration tests.
/// When expanded, the proc macro will: launch a local provider, generate one wallet,
/// deploy the selected contract and create a contract instance.
/// The names for the contract instance and wallet variables must be provided as inputs.
/// This macro can be called multiple times inside a function if the variables names are changed.
/// The same contract can be deployed multiple times as the macro uses deployment with salt.
/// However, if you need to have a shared wallet between macros, the first macro must set the
/// wallet name to `wallet`. The other ones must set the wallet name to `None`.
#[proc_macro]
pub fn setup_contract_test(input: TokenStream) -> TokenStream {
    let test_contract_commands = parse_macro_input!(input as TestContractCommands);

    generate_setup_contract_test_code(test_contract_commands).into()
}
