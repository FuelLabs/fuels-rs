pub mod abigen_bindings {
    pub mod my_contract_mod {
        use ::fuels::{
            accounts::{Account, ViewOnlyAccount},
            core::{
                codec,
                traits::{Parameterize, Tokenizable},
                Configurables,
            },
            programs::{
                contract::{self, ContractCallHandler},
                logs::{self, LogDecoder},
            },
            types::{bech32::Bech32ContractId, errors::Result, AssetId},
        };

        pub struct MyContract<T: Account> {
            contract_id: Bech32ContractId,
            account: T,
            log_decoder: LogDecoder,
        }
        impl<T: Account> MyContract<T> {
            pub fn new(
                contract_id: impl ::core::convert::Into<Bech32ContractId>,
                account: T,
            ) -> Self {
                let contract_id: Bech32ContractId = contract_id.into();
                let log_decoder = LogDecoder::new(logs::log_formatters_lookup(
                    vec![],
                    contract_id.clone().into(),
                ));
                Self {
                    contract_id,
                    account,
                    log_decoder,
                }
            }
            pub fn contract_id(&self) -> &Bech32ContractId {
                &self.contract_id
            }
            pub fn account(&self) -> T {
                self.account.clone()
            }
            pub fn with_account<U: Account>(&self, account: U) -> MyContract<U> {
                MyContract {
                    contract_id: self.contract_id.clone(),
                    account,
                    log_decoder: self.log_decoder.clone(),
                }
            }
            pub async fn get_balances(&self) -> Result<::std::collections::HashMap<AssetId, u64>> {
                ViewOnlyAccount::try_provider(&self.account)?
                    .get_contract_balances(&self.contract_id)
                    .await
                    .map_err(::std::convert::Into::into)
            }
            pub fn methods(&self) -> MyContractMethods<T> {
                MyContractMethods {
                    contract_id: self.contract_id.clone(),
                    account: self.account.clone(),
                    log_decoder: self.log_decoder.clone(),
                }
            }
        }
        pub struct MyContractMethods<T: Account> {
            contract_id: Bech32ContractId,
            account: T,
            log_decoder: LogDecoder,
        }
        impl<T: Account> MyContractMethods<T> {
            #[doc = "Calls the contract's `initialize_counter` function"]
            pub fn initialize_counter(&self, value: u64) -> ContractCallHandler<T, u64> {
                contract::method_hash(
                    self.contract_id.clone(),
                    self.account.clone(),
                    codec::resolve_fn_selector("initialize_counter", &[u64::param_type()]),
                    &[Tokenizable::into_token(value)],
                    self.log_decoder.clone(),
                    false,
                    ABIEncoder::new(EncoderConfig::default()),
                )
            }
            #[doc = "Calls the contract's `increment_counter` function"]
            pub fn increment_counter(&self, value: u64) -> ContractCallHandler<T, u64> {
                contract::method_hash(
                    self.contract_id.clone(),
                    self.account.clone(),
                    codec::resolve_fn_selector("increment_counter", &[u64::param_type()]),
                    &[value.into_token()],
                    self.log_decoder.clone(),
                    false,
                    ABIEncoder::new(EncoderConfig::default()),
                )
            }
        }
        impl<T: Account> contract::SettableContract for MyContract<T> {
            fn id(&self) -> Bech32ContractId {
                self.contract_id.clone()
            }
            fn log_decoder(&self) -> LogDecoder {
                self.log_decoder.clone()
            }
        }
        #[derive(Clone, Debug, Default)]
        pub struct MyContractConfigurables {
            offsets_with_data: ::std::vec::Vec<(u64, ::std::vec::Vec<u8>)>,
        }
        impl MyContractConfigurables {
            pub fn new() -> Self {
                ::std::default::Default::default()
            }
        }
        impl From<MyContractConfigurables> for Configurables {
            fn from(config: MyContractConfigurables) -> Self {
                Configurables::new(config.offsets_with_data)
            }
        }
    }
}
pub use abigen_bindings::my_contract_mod::MyContract;
pub use abigen_bindings::my_contract_mod::MyContractConfigurables;
pub use abigen_bindings::my_contract_mod::MyContractMethods;
