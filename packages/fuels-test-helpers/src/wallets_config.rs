// These constants define the default number of wallets to be setup,
// the number of coins per wallet and the amount per coin
pub const DEFAULT_NUM_WALLETS: u64 = 10;
pub const DEFAULT_NUM_COINS: u64 = 1;
pub const DEFAULT_COIN_AMOUNT: u64 = 1_000_000_000;

#[derive(Debug)]
pub struct WalletsConfig {
    pub num_wallets: u64,
    pub coins_per_wallet: u64,
    pub coin_amount: u64,
}

impl WalletsConfig {
    pub fn new(
        num_wallets: Option<u64>,
        coins_per_wallet: Option<u64>,
        coin_amount: Option<u64>,
    ) -> Self {
        Self {
            num_wallets: num_wallets.unwrap_or(DEFAULT_NUM_WALLETS),
            coins_per_wallet: coins_per_wallet.unwrap_or(DEFAULT_NUM_COINS),
            coin_amount: coin_amount.unwrap_or(DEFAULT_COIN_AMOUNT),
        }
    }

    pub fn new_single(coins: Option<u64>, amount: Option<u64>) -> Self {
        Self {
            num_wallets: 1,
            coins_per_wallet: coins.unwrap_or(DEFAULT_NUM_COINS),
            coin_amount: amount.unwrap_or(DEFAULT_COIN_AMOUNT),
        }
    }
}

impl Default for WalletsConfig {
    fn default() -> Self {
        Self {
            num_wallets: DEFAULT_NUM_WALLETS,
            coins_per_wallet: DEFAULT_NUM_COINS,
            coin_amount: DEFAULT_COIN_AMOUNT,
        }
    }
}
