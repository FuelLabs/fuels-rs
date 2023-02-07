#[allow(clippy::too_many_arguments)]
#[no_implicit_prelude]
pub mod abigen_bindings {
    #[allow(clippy::too_many_arguments)]
    #[no_implicit_prelude]
    pub mod my_contract_test_mod {
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
        #[allow(clippy::enum_variant_names)]
        #[derive(
            Clone,
            Debug,
            Eq,
            PartialEq,
            :: fuels :: macros :: Parameterize,
            :: fuels :: macros :: Tokenizable,
            :: fuels :: macros :: TryFrom,
        )]
        pub enum State {
            A,
            B,
            C,
        }
        #[derive(
            Clone,
            Debug,
            Eq,
            PartialEq,
            :: fuels :: macros :: Parameterize,
            :: fuels :: macros :: Tokenizable,
            :: fuels :: macros :: TryFrom,
        )]
        pub struct MyType {
            pub x: u64,
            pub y: u64,
        }
        #[derive(
            Clone,
            Debug,
            Eq,
            PartialEq,
            :: fuels :: macros :: Parameterize,
            :: fuels :: macros :: Tokenizable,
            :: fuels :: macros :: TryFrom,
        )]
        pub struct Person {
            pub name: ::fuels::types::SizedAsciiString<4usize>,
        }
        pub struct MyContractTest<T> {
            contract_id: ::fuels::types::bech32::Bech32ContractId,
            account: T,
            log_decoder: ::fuels::programs::logs::LogDecoder,
        }
        impl<T: ::fuels::signers::Account + ::fuels::signers::PayFee + ::std::clone::Clone>
            MyContractTest<T>
        where
            ::fuels::types::errors::Error: From<<T as ::fuels::signers::PayFee>::Error>,
        {
            pub fn new(contract_id: ::fuels::types::bech32::Bech32ContractId, account: T) -> Self {
                let log_decoder = ::fuels::programs::logs::LogDecoder {
                    type_lookup: ::fuels::core::utils::log_type_lookup(
                        &[],
                        ::std::option::Option::Some(contract_id.clone()),
                    ),
                };
                Self {
                    contract_id,
                    account,
                    log_decoder,
                }
            }
            pub fn contract_id(&self) -> &::fuels::types::bech32::Bech32ContractId {
                &self.contract_id
            }
            pub fn account(&self) -> T {
                self.account.clone()
            }
            pub fn with_account(&self, mut account: T) -> ::fuels::types::errors::Result<Self> {
                let provider = ::fuels::signers::Account::get_provider(&self.account)?;
                account.set_provider(provider.clone());
                ::std::result::Result::Ok(Self {
                    contract_id: self.contract_id.clone(),
                    account,
                    log_decoder: self.log_decoder.clone(),
                })
            }
            pub async fn get_balances(
                &self,
            ) -> ::fuels::types::errors::Result<
                ::std::collections::HashMap<::std::string::String, u64>,
            > {
                ::fuels::signers::Account::get_provider(&self.account)?
                    .get_contract_balances(&self.contract_id)
                    .await
                    .map_err(::std::convert::Into::into)
            }
            pub fn methods(&self) -> MyContractTestMethods<T> {
                MyContractTestMethods {
                    contract_id: self.contract_id.clone(),
                    account: self.account.clone(),
                    log_decoder: self.log_decoder.clone(),
                }
            }
        }
        pub struct MyContractTestMethods<T> {
            contract_id: ::fuels::types::bech32::Bech32ContractId,
            account: T,
            log_decoder: ::fuels::programs::logs::LogDecoder,
        }
        impl<T: ::fuels::signers::Account + ::fuels::signers::PayFee + ::std::clone::Clone>
            MyContractTestMethods<T>
        {
            #[doc = "Calls the contract's `array_of_enums` function"]
            pub fn array_of_enums(
                &self,
                p: [self::State; 2usize],
            ) -> ::fuels::programs::contract::ContractCallHandler<T, [self::State; 2usize]>
            {
                let provider = ::fuels::signers::Account::get_provider(&self.account)
                    .expect("Provider not set up");
                :: fuels :: programs :: contract :: Contract :: < T > :: method_hash (& provider , self . contract_id . clone () , & self . account , :: fuels :: core :: function_selector :: resolve_fn_selector ("array_of_enums" , & [< [self :: State ; 2usize] as :: fuels :: types :: traits :: Parameterize > :: param_type ()]) , & [:: fuels :: types :: traits :: Tokenizable :: into_token (p)] , self . log_decoder . clone ()) . expect ("method not found (this should never happen)")
            }
            #[doc = "Calls the contract's `array_of_structs` function"]
            pub fn array_of_structs(
                &self,
                p: [self::Person; 2usize],
            ) -> ::fuels::programs::contract::ContractCallHandler<T, [self::Person; 2usize]>
            {
                let provider = ::fuels::signers::Account::get_provider(&self.account)
                    .expect("Provider not set up");
                :: fuels :: programs :: contract :: Contract :: < T > :: method_hash (& provider , self . contract_id . clone () , & self . account , :: fuels :: core :: function_selector :: resolve_fn_selector ("array_of_structs" , & [< [self :: Person ; 2usize] as :: fuels :: types :: traits :: Parameterize > :: param_type ()]) , & [:: fuels :: types :: traits :: Tokenizable :: into_token (p)] , self . log_decoder . clone ()) . expect ("method not found (this should never happen)")
            }
            #[doc = "Calls the contract's `get` function"]
            pub fn get(
                &self,
                x: u64,
                y: u64,
            ) -> ::fuels::programs::contract::ContractCallHandler<T, u64> {
                let provider = ::fuels::signers::Account::get_provider(&self.account)
                    .expect("Provider not set up");
                ::fuels::programs::contract::Contract::<T>::method_hash(
                    &provider,
                    self.contract_id.clone(),
                    &self.account,
                    ::fuels::core::function_selector::resolve_fn_selector(
                        "get",
                        &[
                            <u64 as ::fuels::types::traits::Parameterize>::param_type(),
                            <u64 as ::fuels::types::traits::Parameterize>::param_type(),
                        ],
                    ),
                    &[
                        ::fuels::types::traits::Tokenizable::into_token(x),
                        ::fuels::types::traits::Tokenizable::into_token(y),
                    ],
                    self.log_decoder.clone(),
                )
                .expect("method not found (this should never happen)")
            }
            #[doc = "Calls the contract's `get_alt` function"]
            pub fn get_alt(
                &self,
                t: self::MyType,
            ) -> ::fuels::programs::contract::ContractCallHandler<T, self::MyType> {
                let provider = ::fuels::signers::Account::get_provider(&self.account)
                    .expect("Provider not set up");
                ::fuels::programs::contract::Contract::<T>::method_hash(
                    &provider,
                    self.contract_id.clone(),
                    &self.account,
                    ::fuels::core::function_selector::resolve_fn_selector(
                        "get_alt",
                        &[<self::MyType as ::fuels::types::traits::Parameterize>::param_type()],
                    ),
                    &[::fuels::types::traits::Tokenizable::into_token(t)],
                    self.log_decoder.clone(),
                )
                .expect("method not found (this should never happen)")
            }
            #[doc = "Calls the contract's `get_array` function"]
            pub fn get_array(
                &self,
                p: [u64; 2usize],
            ) -> ::fuels::programs::contract::ContractCallHandler<T, [u64; 2usize]> {
                let provider = ::fuels::signers::Account::get_provider(&self.account)
                    .expect("Provider not set up");
                ::fuels::programs::contract::Contract::<T>::method_hash(
                    &provider,
                    self.contract_id.clone(),
                    &self.account,
                    ::fuels::core::function_selector::resolve_fn_selector(
                        "get_array",
                        &[<[u64; 2usize] as ::fuels::types::traits::Parameterize>::param_type()],
                    ),
                    &[::fuels::types::traits::Tokenizable::into_token(p)],
                    self.log_decoder.clone(),
                )
                .expect("method not found (this should never happen)")
            }
            #[doc = "Calls the contract's `get_counter` function"]
            pub fn get_counter(&self) -> ::fuels::programs::contract::ContractCallHandler<T, u64> {
                let provider = ::fuels::signers::Account::get_provider(&self.account)
                    .expect("Provider not set up");
                ::fuels::programs::contract::Contract::<T>::method_hash(
                    &provider,
                    self.contract_id.clone(),
                    &self.account,
                    ::fuels::core::function_selector::resolve_fn_selector("get_counter", &[]),
                    &[],
                    self.log_decoder.clone(),
                )
                .expect("method not found (this should never happen)")
            }
            #[doc = "Calls the contract's `get_msg_amount` function"]
            pub fn get_msg_amount(
                &self,
            ) -> ::fuels::programs::contract::ContractCallHandler<T, u64> {
                let provider = ::fuels::signers::Account::get_provider(&self.account)
                    .expect("Provider not set up");
                ::fuels::programs::contract::Contract::<T>::method_hash(
                    &provider,
                    self.contract_id.clone(),
                    &self.account,
                    ::fuels::core::function_selector::resolve_fn_selector("get_msg_amount", &[]),
                    &[],
                    self.log_decoder.clone(),
                )
                .expect("method not found (this should never happen)")
            }
            #[doc = "Calls the contract's `get_single` function"]
            pub fn get_single(
                &self,
                x: u64,
            ) -> ::fuels::programs::contract::ContractCallHandler<T, u64> {
                let provider = ::fuels::signers::Account::get_provider(&self.account)
                    .expect("Provider not set up");
                ::fuels::programs::contract::Contract::<T>::method_hash(
                    &provider,
                    self.contract_id.clone(),
                    &self.account,
                    ::fuels::core::function_selector::resolve_fn_selector(
                        "get_single",
                        &[<u64 as ::fuels::types::traits::Parameterize>::param_type()],
                    ),
                    &[::fuels::types::traits::Tokenizable::into_token(x)],
                    self.log_decoder.clone(),
                )
                .expect("method not found (this should never happen)")
            }
            #[doc = "Calls the contract's `increment_counter` function"]
            pub fn increment_counter(
                &self,
                value: u64,
            ) -> ::fuels::programs::contract::ContractCallHandler<T, u64> {
                let provider = ::fuels::signers::Account::get_provider(&self.account)
                    .expect("Provider not set up");
                ::fuels::programs::contract::Contract::<T>::method_hash(
                    &provider,
                    self.contract_id.clone(),
                    &self.account,
                    ::fuels::core::function_selector::resolve_fn_selector(
                        "increment_counter",
                        &[<u64 as ::fuels::types::traits::Parameterize>::param_type()],
                    ),
                    &[::fuels::types::traits::Tokenizable::into_token(value)],
                    self.log_decoder.clone(),
                )
                .expect("method not found (this should never happen)")
            }
            #[doc = "Calls the contract's `initialize_counter` function"]
            pub fn initialize_counter(
                &self,
                value: u64,
            ) -> ::fuels::programs::contract::ContractCallHandler<T, u64> {
                let provider = ::fuels::signers::Account::get_provider(&self.account)
                    .expect("Provider not set up");
                ::fuels::programs::contract::Contract::<T>::method_hash(
                    &provider,
                    self.contract_id.clone(),
                    &self.account,
                    ::fuels::core::function_selector::resolve_fn_selector(
                        "initialize_counter",
                        &[<u64 as ::fuels::types::traits::Parameterize>::param_type()],
                    ),
                    &[::fuels::types::traits::Tokenizable::into_token(value)],
                    self.log_decoder.clone(),
                )
                .expect("method not found (this should never happen)")
            }
            #[doc = "Calls the contract's `new` function"]
            pub fn new(&self) -> ::fuels::programs::contract::ContractCallHandler<T, u64> {
                let provider = ::fuels::signers::Account::get_provider(&self.account)
                    .expect("Provider not set up");
                ::fuels::programs::contract::Contract::<T>::method_hash(
                    &provider,
                    self.contract_id.clone(),
                    &self.account,
                    ::fuels::core::function_selector::resolve_fn_selector("new", &[]),
                    &[],
                    self.log_decoder.clone(),
                )
                .expect("method not found (this should never happen)")
            }
        }
        impl<T: ::fuels::signers::Account + ::fuels::signers::PayFee>
            ::fuels::programs::contract::SettableContract for MyContractTest<T>
        {
            fn id(&self) -> ::fuels::types::bech32::Bech32ContractId {
                self.contract_id.clone()
            }
            fn log_decoder(&self) -> ::fuels::programs::logs::LogDecoder {
                self.log_decoder.clone()
            }
        }
    }
}
pub use abigen_bindings::my_contract_test_mod::MyContractTest;
pub use abigen_bindings::my_contract_test_mod::MyContractTestMethods;
pub use abigen_bindings::my_contract_test_mod::MyType;
pub use abigen_bindings::my_contract_test_mod::Person;
pub use abigen_bindings::my_contract_test_mod::State;
