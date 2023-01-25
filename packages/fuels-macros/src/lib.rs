use proc_macro::TokenStream;
use syn::parse_macro_input;

use crate::{
    abigen_macro::{Abigen, MacroAbigenTargets},
    setup_contract_test_macro::{generate_setup_contract_test_code, TestContractCommands},
};

mod abigen_macro;
mod error;
mod parse_utils;
mod setup_contract_test_macro;
mod utils;

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
///```text
///setup_contract_test!(
///    Wallets("wallet"),
///    Abigen(
///        name = "FooContract",
///        abi = "packages/fuels/tests/contracts/foo_contract"
///    ),
///    Abigen(
///        name = "FooCallerContract",
///        abi = "packages/fuels/tests/contracts/foo_caller_contract"
///    ),
///    Deploy(
///        name = "foo_contract_instance",
///        contract = "FooContract",
///        wallet = "wallet"
///    ),
///    Deploy(
///        name = "foo_caller_contract_instance",
///        contract = "FooCallerContract",
///        wallet = "my_own_wallet"
///    ),
///);
///```
#[proc_macro]
pub fn setup_contract_test(input: TokenStream) -> TokenStream {
    let test_contract_commands = parse_macro_input!(input as TestContractCommands);

    generate_setup_contract_test_code(test_contract_commands)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}
