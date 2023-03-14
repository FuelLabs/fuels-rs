pub mod abigen_bindings {
    pub mod my_contract_name_mod {
        pub struct MyContractName {
            contract_id: Bech32ContractId,
            wallet: WalletUnlocked,
            log_decoder: LogDecoder,
        }

        impl MyContractName {
            pub fn new(contract_id: Bech32ContractId, wallet: WalletUnlocked) -> Self {
                let log_decoder = LogDecoder {
                    type_lookup: logs::log_type_lookup(&[], Some(contract_id.clone().into())),
                };
                Self {
                    contract_id,
                    wallet,
                    log_decoder,
                }
            }
            pub fn contract_id(&self) -> &Bech32ContractId {
                &self.contract_id
            }
            pub fn wallet(&self) -> WalletUnlocked {
                self.wallet.clone()
            }
            pub fn with_wallet(&self, mut wallet: WalletUnlocked) -> Result<Self> {
                let provider = self.wallet.get_provider()?;
                wallet.set_provider(provider.clone());
                Ok(Self {
                    contract_id: self.contract_id.clone(),
                    wallet,
                    log_decoder: self.log_decoder.clone(),
                })
            }
            pub async fn get_balances(&self) -> Result<HashMap<String, u64>> {
                self.wallet
                    .get_provider()?
                    .get_contract_balances(&self.contract_id)
                    .await
                    .map_err(Into::into)
            }
            pub fn methods(&self) -> MyContractNameMethods {
                MyContractNameMethods {
                    contract_id: self.contract_id.clone(),
                    wallet: self.wallet.clone(),
                    log_decoder: self.log_decoder.clone(),
                }
            }
        }

        pub struct MyContractNameMethods {
            contract_id: Bech32ContractId,
            wallet: WalletUnlocked,
            log_decoder: LogDecoder,
        }

        impl MyContractNameMethods {
            #[doc = "Calls the contract's `initialize_counter` function"]
            pub fn initialize_counter(&self, value: u64) -> ContractCallHandler<u64> {
                let provider = self.wallet.get_provider().expect("Provider not set up");
                Contract::method_hash(
                    provider,
                    self.contract_id.clone(),
                    &self.wallet,
                    function_selector::resolve_fn_selector(
                        "initialize_counter",
                        &[u64::param_type()],
                    ),
                    &[value.into_token()],
                    self.log_decoder.clone(),
                    false,
                )
                .expect("method not found (this should never happen)")
            }
            #[doc = "Calls the contract's `increment_counter` function"]
            pub fn increment_counter(&self, value: u64) -> ContractCallHandler<u64> {
                let provider = self.wallet.get_provider().expect("Provider not set up");
                Contract::method_hash(
                    provider,
                    self.contract_id.clone(),
                    &self.wallet,
                    function_selector::resolve_fn_selector(
                        "increment_counter",
                        &[u64::param_type()],
                    ),
                    &[value.into_token()],
                    self.log_decoder.clone(),
                    false,
                )
                .expect("method not found (this should never happen)")
            }
        }

        impl SettableContract for MyContractName {
            fn id(&self) -> Bech32ContractId {
                self.contract_id.clone()
            }
            fn log_decoder(&self) -> LogDecoder {
                self.log_decoder.clone()
            }
        }

        #[derive(Clone, Debug, Default)]
        pub struct MyContractNameConfigurables {
            offsets_with_data: Vec<(u64, Vec<u8>)>,
        }

        impl MyContractNameConfigurables {
            pub fn new() -> Self {
                Default::default()
            }
        }

        impl From<MyContractNameConfigurables> for Configurables {
            fn from(config: MyContractNameConfigurables) -> Self {
                Configurables::new(config.offsets_with_data)
            }
        }
    }
}

pub use abigen_bindings::my_contract_name_mod::MyContractName;
pub use abigen_bindings::my_contract_name_mod::MyContractNameConfigurables;
pub use abigen_bindings::my_contract_name_mod::MyContractNameMethods;
