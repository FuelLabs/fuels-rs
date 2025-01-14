pub mod abigen_bindings {
    pub mod my_contract_mod {
        #[derive(Debug, Clone)]
        pub struct MyContract<A: ::fuels::accounts::Account> {
            contract_id: ::fuels::types::bech32::Bech32ContractId,
            account: A,
            log_decoder: ::fuels::core::codec::LogDecoder,
            encoder_config: ::fuels::core::codec::EncoderConfig,
        }
        impl<A: ::fuels::accounts::Account> MyContract<A> {
            pub fn new(
                contract_id: impl ::core::convert::Into<::fuels::types::bech32::Bech32ContractId>,
                account: A,
            ) -> Self {
                let contract_id: ::fuels::types::bech32::Bech32ContractId = contract_id.into();
                let log_decoder = ::fuels::core::codec::LogDecoder::new(
                    ::fuels::core::codec::log_formatters_lookup(vec![], contract_id.clone().into()),
                );
                let encoder_config = ::fuels::core::codec::EncoderConfig::default();
                Self {
                    contract_id,
                    account,
                    log_decoder,
                    encoder_config,
                }
            }
            pub fn contract_id(&self) -> &::fuels::types::bech32::Bech32ContractId {
                &self.contract_id
            }
            pub fn account(&self) -> A {
                self.account.clone()
            }
            pub fn with_account<U: ::fuels::accounts::Account>(self, account: U) -> MyContract<U> {
                MyContract {
                    contract_id: self.contract_id,
                    account,
                    log_decoder: self.log_decoder,
                    encoder_config: self.encoder_config,
                }
            }
            pub fn with_encoder_config(
                mut self,
                encoder_config: ::fuels::core::codec::EncoderConfig,
            ) -> MyContract<A> {
                self.encoder_config = encoder_config;
                self
            }
            pub async fn get_balances(
                &self,
            ) -> ::fuels::types::errors::Result<
                ::std::collections::HashMap<::fuels::types::AssetId, u64>,
            > {
                ::fuels::accounts::ViewOnlyAccount::try_provider(&self.account)?
                    .get_contract_balances(&self.contract_id)
                    .await
                    .map_err(::std::convert::Into::into)
            }
            pub fn methods(&self) -> MyContractMethods<A> {
                MyContractMethods {
                    contract_id: self.contract_id.clone(),
                    account: self.account.clone(),
                    log_decoder: self.log_decoder.clone(),
                    encoder_config: self.encoder_config.clone(),
                }
            }
        }
        pub struct MyContractMethods<A: ::fuels::accounts::Account> {
            contract_id: ::fuels::types::bech32::Bech32ContractId,
            account: A,
            log_decoder: ::fuels::core::codec::LogDecoder,
            encoder_config: ::fuels::core::codec::EncoderConfig,
        }
        impl<A: ::fuels::accounts::Account> MyContractMethods<A> {
            #[doc = " This method will read the counter from storage, increment it"]
            #[doc = " and write the incremented value to storage"]
            pub fn increment_counter(
                &self,
                value: ::core::primitive::u64,
            ) -> ::fuels::programs::calls::CallHandler<
                A,
                ::fuels::programs::calls::ContractCall,
                ::core::primitive::u64,
            > {
                ::fuels::programs::calls::CallHandler::new_contract_call(
                    self.contract_id.clone(),
                    self.account.clone(),
                    ::fuels::core::codec::encode_fn_selector("increment_counter"),
                    &[::fuels::core::traits::Tokenizable::into_token(value)],
                    self.log_decoder.clone(),
                    false,
                    self.encoder_config.clone(),
                )
            }
            pub fn initialize_counter(
                &self,
                value: ::core::primitive::u64,
            ) -> ::fuels::programs::calls::CallHandler<
                A,
                ::fuels::programs::calls::ContractCall,
                ::core::primitive::u64,
            > {
                ::fuels::programs::calls::CallHandler::new_contract_call(
                    self.contract_id.clone(),
                    self.account.clone(),
                    ::fuels::core::codec::encode_fn_selector("initialize_counter"),
                    &[::fuels::core::traits::Tokenizable::into_token(value)],
                    self.log_decoder.clone(),
                    false,
                    self.encoder_config.clone(),
                )
            }
        }
        impl<A: ::fuels::accounts::Account> ::fuels::programs::calls::ContractDependency for MyContract<A> {
            fn id(&self) -> ::fuels::types::bech32::Bech32ContractId {
                self.contract_id.clone()
            }
            fn log_decoder(&self) -> ::fuels::core::codec::LogDecoder {
                self.log_decoder.clone()
            }
        }
        #[derive(Clone, Debug, Default)]
        pub struct MyContractConfigurables {
            offsets_with_data: ::std::vec::Vec<(u64, ::std::vec::Vec<u8>)>,
            encoder: ::fuels::core::codec::ABIEncoder,
        }
        impl MyContractConfigurables {
            pub fn new(encoder_config: ::fuels::core::codec::EncoderConfig) -> Self {
                Self {
                    encoder: ::fuels::core::codec::ABIEncoder::new(encoder_config),
                    ..::std::default::Default::default()
                }
            }
        }
        impl From<MyContractConfigurables> for ::fuels::core::Configurables {
            fn from(config: MyContractConfigurables) -> Self {
                ::fuels::core::Configurables::new(config.offsets_with_data)
            }
        }
    }
}
pub use abigen_bindings::my_contract_mod::MyContract;
pub use abigen_bindings::my_contract_mod::MyContractConfigurables;
pub use abigen_bindings::my_contract_mod::MyContractMethods;
