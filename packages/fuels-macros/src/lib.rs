use fuels_code_gen::Abigen;
use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};

use crate::{
    abigen::MacroAbigenTargets,
    derive::{
        parameterize::generate_parameterize_impl, tokenizable::generate_tokenizable_impl,
        try_from::generate_try_from_impl,
    },
    setup_program_test::{generate_setup_program_test_code, TestProgramCommands},
};

mod abigen;
mod derive;
mod parse_utils;
mod setup_program_test;

/// Used to generate bindings for Contracts, Scripts and Predicates. Accepts
/// input in the form of `ProgramType(name="MyBindings", abi=ABI_SOURCE)...`
///
/// `ProgramType` is either `Contract`, `Script` or `Predicate`.
///
/// `ABI_SOURCE` is a string literal representing either a path to the JSON ABI
/// file or the contents of the JSON ABI file itself.
///
///```text
/// abigen!(Contract(
///         name = "MyContract",
///         abi = "packages/fuels/tests/contracts/token_ops/out/debug/token_ops-abi.json"
///     ));
///```
///
/// More details can be found in the [`Fuel Rust SDK Book`](https://fuellabs.github.io/fuels-rs/latest)
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

/// Used to reduce boilerplate in integration tests.
///
/// More details can be found in the [`Fuel Rust SDK Book`](https://fuellabs.github.io/fuels-rs/latest)
#[proc_macro]
pub fn setup_program_test(input: TokenStream) -> TokenStream {
    let test_program_commands = parse_macro_input!(input as TestProgramCommands);

    generate_setup_program_test_code(test_program_commands)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

#[proc_macro_derive(Parameterize, attributes(FuelsTypesPath, FuelsCorePath, NoStd))]
pub fn parameterize(stream: TokenStream) -> TokenStream {
    let input = parse_macro_input!(stream as DeriveInput);

    generate_parameterize_impl(input)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

#[proc_macro_derive(Tokenizable, attributes(FuelsTypesPath, FuelsCorePath, NoStd))]
pub fn tokenizable(stream: TokenStream) -> TokenStream {
    let input = parse_macro_input!(stream as DeriveInput);

    generate_tokenizable_impl(input)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

#[proc_macro_derive(TryFrom, attributes(FuelsTypesPath, FuelsCorePath, NoStd))]
pub fn try_from(stream: TokenStream) -> TokenStream {
    let input = parse_macro_input!(stream as DeriveInput);

    generate_try_from_impl(input)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}
