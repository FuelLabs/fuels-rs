use fuel_types::AssetId;

// These constants define the default number of wallets to be setup,
// the number of coins per wallet and the amount per coin
pub const DEFAULT_NUM_WALLETS: u64 = 10;
pub const DEFAULT_NUM_COINS: u64 = 1;
pub const DEFAULT_COIN_AMOUNT: u64 = 1_000_000_000;

#[derive(Debug, Clone)]
pub struct AssetConfig {
    pub id: AssetId,
    pub num_coins: u64,
    pub coin_amount: u64,
}

#[derive(Debug)]
pub struct WalletsConfig {
    num_wallets: u64,
    assets: Vec<AssetConfig>,
}

impl WalletsConfig {
    pub fn new(num_wallets: Option<u64>, num_coins: Option<u64>, coin_amount: Option<u64>) -> Self {
        Self {
            num_wallets: num_wallets.unwrap_or(DEFAULT_NUM_WALLETS),
            assets: vec![AssetConfig {
                id: AssetId::default(),
                num_coins: num_coins.unwrap_or(DEFAULT_NUM_COINS),
                coin_amount: coin_amount.unwrap_or(DEFAULT_COIN_AMOUNT),
            }],
        }
    }

    pub fn new_multiple_assets(num_wallets: u64, assets: Vec<AssetConfig>) -> Self {
        Self {
            num_wallets,
            assets,
        }
    }

    pub fn num_wallets(&self) -> u64 {
        self.num_wallets
    }

    pub fn assets(&self) -> &[AssetConfig] {
        &self.assets[..]
    }
}

impl Default for WalletsConfig {
    fn default() -> Self {
        Self {
            num_wallets: DEFAULT_NUM_WALLETS,
            assets: vec![AssetConfig {
                id: AssetId::default(),
                num_coins: DEFAULT_NUM_COINS,
                coin_amount: DEFAULT_COIN_AMOUNT,
            }],
        }
    }
}
