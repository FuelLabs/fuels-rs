use fuel_types::AssetId;

// ANCHOR: default_tx_parameters
pub const DEFAULT_GAS_LIMIT: u64 = 1_000_000;
pub const DEFAULT_GAS_PRICE: u64 = 0;
pub const DEFAULT_MATURITY: u64 = 0;
// ANCHOR_END: default_tx_parameters

// ANCHOR: default_call_parameters
// Limit for the actual contract call
pub const DEFAULT_FORWARDED_GAS: u64 = 1_000_000;
// Lower limit when querying spendable UTXOs
pub const DEFAULT_SPENDABLE_COIN_AMOUNT: u64 = 1_000_000;
// Bytes representation of the asset ID of the "base" asset used for gas fees.
pub const BASE_ASSET_ID: AssetId = AssetId::new([0u8; 32]);
// ANCHOR_END: default_call_parameters

pub const DEFAULT_GAS_ESTIMATION_TOLERANCE: f64 = 0.2;
pub const GAS_PRICE_FACTOR: u64 = 1_000_000_000;
pub const MAX_GAS_PER_TX: u64 = 100_000_000;

pub const CONTRACT_ID_SWAY_NATIVE_TYPE: &str = "ContractId";
pub const ADDRESS_SWAY_NATIVE_TYPE: &str = "Address";
