use fuel_tx::Word;
use fuel_types::AssetId;

pub const ENUM_DISCRIMINANT_BYTE_WIDTH: usize = 8;
pub const WORD_SIZE: usize = core::mem::size_of::<Word>();

// ANCHOR: default_call_parameters
pub const DEFAULT_CALL_PARAMS_AMOUNT: u64 = 0;
// Bytes representation of the asset ID of the "base" asset used for gas fees.
pub const BASE_ASSET_ID: AssetId = AssetId::BASE;
// ANCHOR_END: default_call_parameters

pub const DEFAULT_GAS_ESTIMATION_TOLERANCE: f64 = 0.2;
