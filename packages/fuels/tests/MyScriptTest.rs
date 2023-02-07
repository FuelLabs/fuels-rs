// #[allow(clippy::too_many_arguments)]
// #[no_implicit_prelude]
// pub mod abigen_bindings {
//     #[allow(clippy::too_many_arguments)]
//     #[no_implicit_prelude]
//     pub mod my_script_test_mod {
//         use ::std::{clone::Clone, convert::{Into, TryFrom, From}, format, iter::IntoIterator, iter::Iterator, marker::Sized, panic, vec, string::ToString};
//
//         #[derive(Clone, Debug, Eq, PartialEq, ::fuels::macros::Parameterize, ::fuels::macros::Tokenizable, ::fuels::macros::TryFrom)]
//         pub struct Bimbam<> {
//             pub val: u64,
//         }
//
//         #[derive(Clone, Debug, Eq, PartialEq, ::fuels::macros::Parameterize, ::fuels::macros::Tokenizable, ::fuels::macros::TryFrom)]
//         pub struct SugarySnack<> {
//             pub twix: u64,
//             pub mars: u64,
//         }
//
//         #[derive(Debug)]
//         pub struct MyScriptTest<T> {
//             account: T,
//             binary_filepath: ::std::string::String,
//             log_decoder: ::fuels::programs::logs::LogDecoder,
//         }
//
//         impl<T: ::fuels::signers::Account + ::fuels::signers::PayFee + ::std::clone::Clone> MyScriptTest<T> where ::fuels::types::errors::Error: From<<T as ::fuels::signers::PayFee>::Error> {
//             pub fn new(account: T, binary_filepath: &str) -> Self { Self { account, binary_filepath: binary_filepath.to_string(), log_decoder: ::fuels::programs::logs::LogDecoder { type_lookup: ::fuels::core::utils::log_type_lookup(&[], ::std::option::Option::None) } } }
//             #[doc = "Run the script's `main` function with the provided arguments"]
//             pub fn main(&self, bim: self::Bimbam, bam: self::SugarySnack) -> ::fuels::programs::script_calls::ScriptCallHandler<T, self::Bimbam> {
//                 let script_binary = ::std::fs::read(&self.binary_filepath).expect("Could not read from binary filepath");
//                 let encoded_args = ::fuels::core::abi_encoder::ABIEncoder::encode(&[::fuels::types::traits::Tokenizable::into_token(bim), ::fuels::types::traits::Tokenizable::into_token(bam)]).expect("Cannot encode script arguments");
//                 let provider = ::fuels::signers::Account::get_provider(&self.account).expect("Provider not set up").clone();
//                 ::fuels::programs::script_calls::ScriptCallHandler::new(script_binary, encoded_args, self.account.clone(), provider, self.log_decoder.clone())
//             }
//         }
//     }
// }
//
// pub use abigen_bindings::my_script_test_mod::Bimbam;
// pub use abigen_bindings::my_script_test_mod::MyScriptTest;
// pub use abigen_bindings::my_script_test_mod::SugarySnack;
