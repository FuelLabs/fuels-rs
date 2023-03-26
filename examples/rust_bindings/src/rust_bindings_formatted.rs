pub mod abigen_bindings {
    pub mod my_contract_mod {
        pub struct MyContract<T: Account> {
            contract_id: Bech32ContractId,
            account: T,
            log_decoder: LogDecoder,
        }
        impl<T: Account> MyContract<T> {
            pub fn new(contract_id: Bech32ContractId, account: T) -> Self {
                let log_decoder = LogDecoder {
                    type_lookup: logs::log_type_lookup(&[], contract_id.clone().into()),
                };
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
            pub fn with_account<U: Account>(&self, mut account: U) -> Result<MyContract<U>> {
                let provider = ViewOnlyAccount::try_provider(&self.account)?;
                account.set_provider(provider.clone());
                Ok(MyContract {
                    contract_id: self.contract_id.clone(),
                    account,
                    log_decoder: self.log_decoder.clone(),
                })
            }
            pub async fn get_balances(&self) -> Result<HashMap<String, u64>> {
                ViewOnlyAccount::try_provider(&self.account)?
                    .get_contract_balances(&self.contract_id)
                    .await
                    .map_err(Into::into)
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
                Contract::method_hash(
                    self.contract_id.clone(),
                    self.account,
                    function_selector::resolve_fn_selector(
                        "initialize_counter",
                        &[<u64 as Parameterize>::param_type()],
                    ),
                    &[Tokenizable::into_token(value)],
                    self.log_decoder.clone(),
                    false,
                )
                .expect("method not found (this should never happen)")
            }
            #[doc = "Calls the contract's `increment_counter` function"]
            pub fn increment_counter(&self, value: u64) -> ContractCallHandler<T, u64> {
                Contract::method_hash(
                    self.contract_id.clone(),
                    self.account,
                    function_selector::resolve_fn_selector(
                        "increment_counter",
                        &[<u64 as Parameterize>::param_type()],
                    ),
                    &[Tokenizable::into_token(value)],
                    self.log_decoder.clone(),
                    false,
                )
                .expect("method not found (this should never happen)")
            }
        }
        impl<T: Account> SettableContract for MyContract<T> {
            fn id(&self) -> Bech32ContractId {
                self.contract_id.clone()
            }
            fn log_decoder(&self) -> LogDecoder {
                self.log_decoder.clone()
            }
        }
        #[derive(Clone, Debug, Default)]
        pub struct MyContractConfigurables {
            offsets_with_data: Vec<(u64, Vec<u8>)>,
        }
        impl MyContractConfigurables {
            pub fn new() -> Self {
                Default::default()
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
