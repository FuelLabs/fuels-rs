use fuel_tx::Word;
use fuel_types::AssetId;

pub const DEFAULT_GAS_LIMIT: u64 = 1_000_000;
pub const DEFAULT_GAS_PRICE: u64 = 0;
pub const DEFAULT_BYTE_PRICE: u64 = 0;
pub const DEFAULT_MATURITY: u64 = 0;

pub const WORD_SIZE: usize = core::mem::size_of::<Word>();

// This constant is used as the lower limit when querying spendable UTXOs
pub const DEFAULT_SPENDABLE_COIN_AMOUNT: u64 = 1_000_000;

// This constant is the bytes representation of the asset ID of
// the "base" asset used for gas fees.
pub const BASE_ASSET_ID: AssetId = AssetId::new([0u8; 32]);
