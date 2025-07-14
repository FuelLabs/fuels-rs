use std::{env, fs, path::Path};

use fuels_code_gen::Abigen;
use proc_macro::TokenStream;
use syn::{DeriveInput, parse_macro_input};

use crate::{
    abigen::MacroAbigenTargets,
    derive::{
        parameterize::generate_parameterize_impl, tokenizable::generate_tokenizable_impl,
        try_from::generate_try_from_impl,
    },
    setup_program_test::{TestProgramCommands, generate_setup_program_test_code},
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
///         abi = "packages/fuels/tests/contracts/token_ops/out/release/token_ops-abi.json"
///     ));
///```
///
/// More details can be found in the [`Fuel Rust SDK Book`](https://fuellabs.github.io/fuels-rs/latest)
#[proc_macro]
pub fn abigen(input: TokenStream) -> TokenStream {
    let targets = parse_macro_input!(input as MacroAbigenTargets);

    Abigen::generate(targets.into(), false)
        .expect("abigen generation failed")
        .into()
}

#[proc_macro]
pub fn wasm_abigen(input: TokenStream) -> TokenStream {
    let targets = parse_macro_input!(input as MacroAbigenTargets);

    Abigen::generate(targets.into(), true)
        .expect("abigen generation failed")
        .into()
}

/// Used to reduce boilerplate in integration tests.
///
/// More details can be found in the [`Fuel Rust SDK Book`](https://fuellabs.github.io/fuels-rs/latest)
#[proc_macro]
pub fn setup_program_test(input: TokenStream) -> TokenStream {
    let test_program_commands = parse_macro_input!(input as TestProgramCommands);

    // Generate the TokenStream
    let tokens = match generate_setup_program_test_code(test_program_commands) {
        Ok(toks) => toks,
        Err(err) => return err.to_compile_error().into(),
    };

    // Only dump when DEBUG_SETUP=1 is set in the build environment
    if env::var_os("DEBUG_SETUP").is_some() {
        let target_dir = env::var("CARGO_TARGET_DIR").unwrap_or_else(|_| "target".into());
        let profile   = env::var("PROFILE").unwrap_or_else(|_| "debug".into());
        let out_dir   = Path::new(&target_dir)
            .join(&profile)
            .join("generated_tests");

        if let Err(e) = fs::create_dir_all(&out_dir) {
            panic!("failed to create debug dir {:?}: {}", out_dir, e);
        }
        let file_path = out_dir.join("setup_program_test.rs");
        if let Err(e) = fs::write(&file_path, tokens.to_string()) {
            panic!("failed to write debug file {:?}: {}", file_path, e);
        }

        eprintln!("[setup_program_test] Wrote generated code to {:?}", file_path);
    }

    tokens.into()
}

#[proc_macro_derive(Parameterize, attributes(FuelsTypesPath, FuelsCorePath, NoStd, Ignore))]
pub fn parameterize(stream: TokenStream) -> TokenStream {
    let input = parse_macro_input!(stream as DeriveInput);

    generate_parameterize_impl(input)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

#[proc_macro_derive(Tokenizable, attributes(FuelsTypesPath, FuelsCorePath, NoStd, Ignore))]
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
