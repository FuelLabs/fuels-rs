#[allow(clippy::too_many_arguments)]
#[no_implicit_prelude]
pub mod abigen_bindings {
    #[allow(clippy::too_many_arguments)]
    #[no_implicit_prelude]
    pub mod my_predicate_test_mod {
        use ::std::{
            clone::Clone,
            convert::{From, Into, TryFrom},
            format,
            iter::IntoIterator,
            iter::Iterator,
            marker::Sized,
            panic,
            string::ToString,
            vec,
        };
        #[derive(Debug, Clone)]
        pub struct MyPredicateTest {
            address: ::fuels::types::bech32::Bech32Address,
            code: ::std::vec::Vec<u8>,
            data: ::fuels::core::abi_encoder::UnresolvedBytes,
            provider: ::std::option::Option<::fuels::prelude::Provider>,
        }
        impl MyPredicateTest {
            pub fn get_predicate(&self) -> ::fuels::programs::predicate::Predicate {
                ::fuels::programs::predicate::Predicate {
                    address: self.address.clone(),
                    code: self.code.clone(),
                    data: self.data.clone(),
                    provider: self.provider.clone(),
                }
            }
            pub fn new(code: ::std::vec::Vec<u8>) -> Self {
                let address: ::fuels::types::Address =
                    (*::fuels::tx::Contract::root_from_code(&code)).into();
                Self {
                    address: address.clone().into(),
                    code,
                    data: ::fuels::core::abi_encoder::UnresolvedBytes::new(),
                    provider: ::std::option::Option::None,
                }
            }
            pub fn load_from(file_path: &str) -> ::fuels::types::errors::Result<Self> {
                ::std::result::Result::Ok(Self::new(::std::fs::read(file_path)?))
            }
            #[doc = "Run the predicate's encode function with the provided arguments"]
            pub fn encode_data(&self, a: u64) -> Self {
                let data = ::fuels::core::abi_encoder::ABIEncoder::encode(&[
                    ::fuels::types::traits::Tokenizable::into_token(a),
                ])
                .expect("Cannot encode predicate data");
                Self {
                    address: self.address.clone(),
                    code: self.code.clone(),
                    data,
                    provider: self.provider.clone(),
                }
            }
        }
    }
}
pub use abigen_bindings::my_predicate_test_mod::MyPredicateTest;
