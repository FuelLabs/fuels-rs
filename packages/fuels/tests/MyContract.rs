#[allow(clippy::too_many_arguments)]
#[no_implicit_prelude]
pub mod abigen_bindings {
    #[allow(clippy::too_many_arguments)]
    #[no_implicit_prelude]
    pub mod my_contract_mod {
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
        pub struct Person {
            pub name: ::fuels::types::SizedAsciiString<4usize>,
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
        pub struct MyContract {
            contract_id: ::fuels::types::bech32::Bech32ContractId,
            wallet: ::fuels::signers::wallet::WalletUnlocked,
            log_decoder: ::fuels::programs::logs::LogDecoder,
        }
        impl MyContract {
            pub fn new(
                contract_id: ::fuels::types::bech32::Bech32ContractId,
                wallet: ::fuels::signers::wallet::WalletUnlocked,
            ) -> Self {
                let log_decoder = ::fuels::programs::logs::LogDecoder {
                    type_lookup: ::fuels::core::utils::log_type_lookup(
                        &[],
                        ::std::option::Option::Some(contract_id.clone()),
                    ),
                };
                Self {
                    contract_id,
                    wallet,
                    log_decoder,
                }
            }
            pub fn contract_id(&self) -> &::fuels::types::bech32::Bech32ContractId {
                &self.contract_id
            }
            pub fn wallet(&self) -> ::fuels::signers::wallet::WalletUnlocked {
                self.wallet.clone()
            }
            pub fn with_wallet(
                &self,
                mut wallet: ::fuels::signers::wallet::WalletUnlocked,
            ) -> ::fuels::types::errors::Result<Self> {
                let provider = self.wallet.get_provider()?;
                wallet.set_provider(provider.clone());
                ::std::result::Result::Ok(Self {
                    contract_id: self.contract_id.clone(),
                    wallet: wallet,
                    log_decoder: self.log_decoder.clone(),
                })
            }
            pub async fn get_balances(
                &self,
            ) -> ::fuels::types::errors::Result<
                ::std::collections::HashMap<::std::string::String, u64>,
            > {
                self.wallet
                    .get_provider()?
                    .get_contract_balances(&self.contract_id)
                    .await
                    .map_err(Into::into)
            }
            pub fn methods(&self) -> MyContractMethods {
                MyContractMethods {
                    contract_id: self.contract_id.clone(),
                    wallet: self.wallet.clone(),
                    log_decoder: self.log_decoder.clone(),
                }
            }
        }
        pub struct MyContractMethods {
            contract_id: ::fuels::types::bech32::Bech32ContractId,
            wallet: ::fuels::signers::wallet::WalletUnlocked,
            log_decoder: ::fuels::programs::logs::LogDecoder,
        }
        impl MyContractMethods {
            #[doc = "Calls the contract's `array_of_enums` function"]
            pub fn array_of_enums(
                &self,
                p: [self::State; 2usize],
            ) -> ::fuels::programs::contract::ContractCallHandler<[self::State; 2usize]>
            {
                let provider = self.wallet.get_provider().expect("Provider not set up");
                :: fuels :: programs :: contract :: Contract :: method_hash (& provider , self . contract_id . clone () , & self . wallet , :: fuels :: core :: function_selector :: resolve_fn_selector ("array_of_enums" , & [< [self :: State ; 2usize] as :: fuels :: types :: traits :: Parameterize > :: param_type ()]) , & [:: fuels :: types :: traits :: Tokenizable :: into_token (p)] , self . log_decoder . clone ()) . expect ("method not found (this should never happen)")
            }
            #[doc = "Calls the contract's `array_of_structs` function"]
            pub fn array_of_structs(
                &self,
                p: [self::Person; 2usize],
            ) -> ::fuels::programs::contract::ContractCallHandler<[self::Person; 2usize]>
            {
                let provider = self.wallet.get_provider().expect("Provider not set up");
                :: fuels :: programs :: contract :: Contract :: method_hash (& provider , self . contract_id . clone () , & self . wallet , :: fuels :: core :: function_selector :: resolve_fn_selector ("array_of_structs" , & [< [self :: Person ; 2usize] as :: fuels :: types :: traits :: Parameterize > :: param_type ()]) , & [:: fuels :: types :: traits :: Tokenizable :: into_token (p)] , self . log_decoder . clone ()) . expect ("method not found (this should never happen)")
            }
            #[doc = "Calls the contract's `get` function"]
            pub fn get(
                &self,
                x: u64,
                y: u64,
            ) -> ::fuels::programs::contract::ContractCallHandler<u64> {
                let provider = self.wallet.get_provider().expect("Provider not set up");
                ::fuels::programs::contract::Contract::method_hash(
                    &provider,
                    self.contract_id.clone(),
                    &self.wallet,
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
            ) -> ::fuels::programs::contract::ContractCallHandler<self::MyType> {
                let provider = self.wallet.get_provider().expect("Provider not set up");
                ::fuels::programs::contract::Contract::method_hash(
                    &provider,
                    self.contract_id.clone(),
                    &self.wallet,
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
            ) -> ::fuels::programs::contract::ContractCallHandler<[u64; 2usize]> {
                let provider = self.wallet.get_provider().expect("Provider not set up");
                ::fuels::programs::contract::Contract::method_hash(
                    &provider,
                    self.contract_id.clone(),
                    &self.wallet,
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
            pub fn get_counter(&self) -> ::fuels::programs::contract::ContractCallHandler<u64> {
                let provider = self.wallet.get_provider().expect("Provider not set up");
                ::fuels::programs::contract::Contract::method_hash(
                    &provider,
                    self.contract_id.clone(),
                    &self.wallet,
                    ::fuels::core::function_selector::resolve_fn_selector("get_counter", &[]),
                    &[],
                    self.log_decoder.clone(),
                )
                .expect("method not found (this should never happen)")
            }
            #[doc = "Calls the contract's `get_msg_amount` function"]
            pub fn get_msg_amount(&self) -> ::fuels::programs::contract::ContractCallHandler<u64> {
                let provider = self.wallet.get_provider().expect("Provider not set up");
                ::fuels::programs::contract::Contract::method_hash(
                    &provider,
                    self.contract_id.clone(),
                    &self.wallet,
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
            ) -> ::fuels::programs::contract::ContractCallHandler<u64> {
                let provider = self.wallet.get_provider().expect("Provider not set up");
                ::fuels::programs::contract::Contract::method_hash(
                    &provider,
                    self.contract_id.clone(),
                    &self.wallet,
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
            ) -> ::fuels::programs::contract::ContractCallHandler<u64> {
                let provider = self.wallet.get_provider().expect("Provider not set up");
                ::fuels::programs::contract::Contract::method_hash(
                    &provider,
                    self.contract_id.clone(),
                    &self.wallet,
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
            ) -> ::fuels::programs::contract::ContractCallHandler<u64> {
                let provider = self.wallet.get_provider().expect("Provider not set up");
                ::fuels::programs::contract::Contract::method_hash(
                    &provider,
                    self.contract_id.clone(),
                    &self.wallet,
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
            pub fn new(&self) -> ::fuels::programs::contract::ContractCallHandler<u64> {
                let provider = self.wallet.get_provider().expect("Provider not set up");
                ::fuels::programs::contract::Contract::method_hash(
                    &provider,
                    self.contract_id.clone(),
                    &self.wallet,
                    ::fuels::core::function_selector::resolve_fn_selector("new", &[]),
                    &[],
                    self.log_decoder.clone(),
                )
                .expect("method not found (this should never happen)")
            }
        }
        impl ::fuels::programs::contract::SettableContract for MyContract {
            fn id(&self) -> ::fuels::types::bech32::Bech32ContractId {
                self.contract_id.clone()
            }
            fn log_decoder(&self) -> ::fuels::programs::logs::LogDecoder {
                self.log_decoder.clone()
            }
        }
    }
}
pub use abigen_bindings::my_contract_mod::MyContract;
pub use abigen_bindings::my_contract_mod::MyContractMethods;
pub use abigen_bindings::my_contract_mod::MyType;
pub use abigen_bindings::my_contract_mod::Person;
pub use abigen_bindings::my_contract_mod::State;
