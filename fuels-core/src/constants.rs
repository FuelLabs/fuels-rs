use fuel_tx::Word;

pub const DEFAULT_GAS_LIMIT: u64 = 1_000_000;
pub const DEFAULT_GAS_PRICE: u64 = 0;
pub const DEFAULT_BYTE_PRICE: u64 = 0;

pub const WORD_SIZE: usize = core::mem::size_of::<Word>();

// This constant is used to determine the amount in the 1 UTXO
// when initializing wallets for now.
pub const DEFAULT_COIN_AMOUNT: u64 = 1;
pub const DEFAULT_INITIAL_BALANCE: u64 = 100;

// This constant is the bytes representation of the asset ID of
// Ethereum right now, the "native" token used for gas fees.
pub const NATIVE_ASSET_ID: [u8; 32] = [0u8; 32];
