pub mod abigen_bindings {
    pub mod my_contract_mod {
        // Changing the ABI file triggers a recompilation of the `abigen!` macro.
        const _: &[u8] = include_bytes!("/path/to/examples/rust_bindings/src/abi.json");
        #[derive(Debug, Clone)]
        pub struct MyContract<A = ()> {
            contract_id: ::fuels::types::ContractId,
            account: A,
            log_decoder: ::fuels::core::codec::LogDecoder,
            encoder_config: ::fuels::core::codec::EncoderConfig,
        }
        impl MyContract {
            pub const METHODS: MyContractMethodVariants = MyContractMethodVariants;
        }
        impl<A> MyContract<A> {
            pub fn new(contract_id: ::fuels::types::ContractId, account: A) -> Self {
                let log_decoder = ::fuels::core::codec::LogDecoder::new(
                    ::fuels::core::codec::log_formatters_lookup(vec![], contract_id.clone().into()),
                    ::std::collections::HashMap::from([]),
                );
                let encoder_config = ::fuels::core::codec::EncoderConfig::default();
                Self {
                    contract_id,
                    account,
                    log_decoder,
                    encoder_config,
                }
            }
            pub fn contract_id(&self) -> ::fuels::types::ContractId {
                self.contract_id
            }
            pub fn account(&self) -> &A {
                &self.account
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
            >
            where
                A: ::fuels::accounts::ViewOnlyAccount,
            {
                ::fuels::accounts::ViewOnlyAccount::try_provider(&self.account)?
                    .get_contract_balances(&self.contract_id)
                    .await
                    .map_err(::std::convert::Into::into)
            }
            pub fn methods(&self) -> MyContractMethods<A>
            where
                A: Clone,
            {
                MyContractMethods {
                    contract_id: self.contract_id.clone(),
                    account: self.account.clone(),
                    log_decoder: self.log_decoder.clone(),
                    encoder_config: self.encoder_config.clone(),
                }
            }
        }
        pub struct MyContractMethods<A> {
            contract_id: ::fuels::types::ContractId,
            account: A,
            log_decoder: ::fuels::core::codec::LogDecoder,
            encoder_config: ::fuels::core::codec::EncoderConfig,
        }
        impl<A: ::fuels::accounts::Account + Clone> MyContractMethods<A> {
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
        }
        impl<A> ::fuels::programs::calls::ContractDependency for MyContract<A> {
            fn id(&self) -> ::fuels::types::ContractId {
                self.contract_id
            }
            fn log_decoder(&self) -> ::fuels::core::codec::LogDecoder {
                self.log_decoder.clone()
            }
        }
        #[derive(Clone, Debug, Default)]
        pub struct MyContractConfigurables {
            offsets_with_data: ::std::vec::Vec<::fuels::core::Configurable>,
            encoder: ::fuels::core::codec::ABIEncoder,
        }
        impl MyContractConfigurables {
            pub fn new(encoder_config: ::fuels::core::codec::EncoderConfig) -> Self {
                Self {
                    encoder: ::fuels::core::codec::ABIEncoder::new(encoder_config),
                    ..::std::default::Default::default()
                }
            }
            // A `with_XXX` method is generated here for every `configurable` in the ABI.
            // This contract has none, so the list is empty.
        }
        impl From<MyContractConfigurables> for ::fuels::core::Configurables {
            fn from(config: MyContractConfigurables) -> Self {
                ::fuels::core::Configurables::new(config.offsets_with_data)
            }
        }
        impl From<MyContractConfigurables> for ::std::vec::Vec<::fuels::core::Configurable> {
            fn from(
                config: MyContractConfigurables,
            ) -> ::std::vec::Vec<::fuels::core::Configurable> {
                config.offsets_with_data
            }
        }
        #[derive(Debug, Clone, Copy)]
        pub struct MyContractMethodVariants;
        impl MyContractMethodVariants {
            pub const fn initialize_counter(&self) -> ::fuels::types::MethodDescriptor {
                ::fuels::types::MethodDescriptor {
                    name: "initialize_counter",
                    fn_selector: b"\0\0\0\0\0\0\0\x12initialize_counter",
                }
            }
            pub const fn increment_counter(&self) -> ::fuels::types::MethodDescriptor {
                ::fuels::types::MethodDescriptor {
                    name: "increment_counter",
                    fn_selector: b"\0\0\0\0\0\0\0\x11increment_counter",
                }
            }
            pub const fn iter(&self) -> [::fuels::types::MethodDescriptor; 2usize] {
                [Self.initialize_counter(), Self.increment_counter()]
            }
        }
    }
}
pub use abigen_bindings::my_contract_mod::MyContract;
pub use abigen_bindings::my_contract_mod::MyContractConfigurables;
pub use abigen_bindings::my_contract_mod::MyContractMethodVariants;
pub use abigen_bindings::my_contract_mod::MyContractMethods;
