pub struct MyContract {
    contract_id: ContractId,
    wallet: WalletUnlocked,
}
impl MyContract {
    pub fn new(contract_id: String, wallet: WalletUnlocked) -> Self {
        let contract_id = ContractId::from_str(&contract_id).expect("Invalid contract id");
        Self {
            contract_id,
            wallet,
        }
    }
    #[doc = "Calls the contract's `initialize_counter` (0x00000000ab64e5f2) function"]
    pub fn initialize_counter(&self, arg: u64) -> ContractCallHandler<u64> {
        Contract::method_hash(
            &self.wallet.get_provider().expect("Provider not set up"),
            self.contract_id,
            &self.wallet,
            [0, 0, 0, 0, 171, 100, 229, 242],
            &[ParamType::U64],
            &[arg.into_token()],
        )
        .expect("method not found (this should never happen)")
    }
    #[doc = "Calls the contract's `increment_counter` (0x00000000faf90dd3) function"]
    pub fn increment_counter(&self, arg: u64) -> ContractCallHandler<u64> {
        Contract::method_hash(
            &self.wallet.get_provider().expect("Provider not set up"),
            self.contract_id,
            &self.wallet,
            [0, 0, 0, 0, 250, 249, 13, 211],
            &[ParamType::U64],
            &[arg.into_token()],
        )
        .expect("method not found (this should never happen)")
    }
}
