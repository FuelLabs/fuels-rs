extern crate core;

use proc_macro::TokenStream;

use syn::parse_macro_input;

use fuels_core::code_gen::abigen::Abigen;

use crate::abigen_macro::MacroAbigenTargets;
use crate::setup_contract_test_macro::{generate_setup_contract_test_code, TestContractCommands};

mod abigen_macro;
mod parse_utils;
mod setup_contract_test_macro;

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

/// Used to reduce boilerplate in integration tests. Accepts inputs in the form
/// of `COMMAND(ARG...)...`
///
/// `COMMAND` is either `Wallets`, `Abigen` or `Deploy`.
///
/// `ARG` is either a:
/// * name-value (e.g. `name="MyContract"`), or,
/// * a literal (e.g. `"some_str_literal"`, `true`, `5`, ...)
///
/// Available `COMMAND`s:
/// ---------------------
/// Wallets
/// ---
///
/// Example: `Wallets("a_wallet", "another_wallet"...)`
///
/// Description: Launches a local provider and generates wallets with names
/// taken from the provided `ARG`s.
///
/// Cardinality: 0 or 1.
///
/// Abigen
/// ---
///
/// Example: `Abigen(name="MyContract", abi="some_folder")`
///
/// Description: Generates the contract bindings under the name `name`. `abi`
/// should point to the folder containing the `out` directory of the forc build.
///
/// Cardinality: 0 or N.
///
/// Deploy
/// ---
///
/// Example: `Deploy(name="instance_name", contract="MyContract", wallet="a_wallet")`
///
/// Description: Deploys the `contract` (with salt) using `wallet`. Will create
/// a contract instance accessible via `name`.
/// Due to salt usage, the same contract can be deployed multiple times.
/// Requires that an `Abigen` command be present with `name` equal to
/// `contract`.
/// `wallet` can either be one of the wallets in the `Wallets` `COMMAND` or the
/// name of a wallet you've previously generated yourself.
///
/// Cardinality: 0 or N.
///
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

    generate_setup_contract_test_code(test_contract_commands).into()
}
