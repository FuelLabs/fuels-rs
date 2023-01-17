pub mod abigen_bindings {
    pub mod my_contract_name_mod {
        pub struct MyContractName {
            contract_id: Bech32ContractId,
            wallet: WalletUnlocked,
        }
        impl MyContractName {
            pub fn new(contract_id: Bech32ContractId, wallet: WalletUnlocked) -> Self {
                Self {
                    contract_id,
                    wallet,
                }
            }
            pub fn contract_id(&self) -> &Bech32ContractId {
                &self.contract_id
            }
            pub fn wallet(&self) -> WalletUnlocked {
                self.wallet.clone()
            }
            pub fn with_wallet(&self, mut wallet: WalletUnlocked) -> Result<Self, Error> {
                let provider = self.wallet.get_provider()?;
                wallet.set_provider(provider.clone());
                Ok(Self {
                    contract_id: self.contract_id.clone(),
                    wallet,
                })
            }
            pub async fn get_balances(&self) -> Result<HashMap<String, u64>, Error> {
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
                    logs_map: get_logs_hashmap(&[], Some(self.contract_id.clone())),
                }
            }
        }
        pub struct MyContractNameMethods {
            contract_id: Bech32ContractId,
            wallet: WalletUnlocked,
            logs_map: HashMap<(Bech32ContractId, u64), ParamType>,
        }
        impl MyContractNameMethods {
            #[doc = "Calls the contract's `initialize_counter` function"]
            pub fn initialize_counter(&self, value: u64) -> ContractCallHandler<u64> {
                let provider = self.wallet.get_provider().expect("Provider not set up");
                let log_decoder = LogDecoder {
                    logs_map: self.logs_map.clone(),
                };
                Contract::method_hash(
                    provider,
                    self.contract_id.clone(),
                    &self.wallet,
                    resolve_fn_selector(
                        "initialize_counter",
                        &[<u64 as Parameterize>::param_type()],
                    ),
                    &[Tokenizable::into_token(value)],
                    log_decoder,
                )
                .expect("method not found (this should never happen)")
            }
            #[doc = "Calls the contract's `increment_counter` function"]
            pub fn increment_counter(&self, value: u64) -> ContractCallHandler<u64> {
                let provider = self.wallet.get_provider().expect("Provider not set up");
                let log_decoder = LogDecoder {
                    logs_map: self.logs_map.clone(),
                };
                Contract::method_hash(
                    provider,
                    self.contract_id.clone(),
                    &self.wallet,
                    resolve_fn_selector(
                        "increment_counter",
                        &[<u64 as Parameterize>::param_type()],
                    ),
                    &[Tokenizable::into_token(value)],
                    log_decoder,
                )
                .expect("method not found (this should never happen)")
            }
        }
    }
}
pub use abigen_bindings::my_contract_name_mod::MyContractName;
pub use abigen_bindings::my_contract_name_mod::MyContractNameMethods;
