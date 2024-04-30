use fuel_tx::Word;

pub const ENUM_DISCRIMINANT_BYTE_WIDTH: usize = 8;
pub const WORD_SIZE: usize = core::mem::size_of::<Word>();

// ANCHOR: default_call_parameters
pub const DEFAULT_CALL_PARAMS_AMOUNT: u64 = 0;
// ANCHOR_END: default_call_parameters

pub const DEFAULT_GAS_ESTIMATION_TOLERANCE: f64 = 0.2;
pub const DEFAULT_GAS_ESTIMATION_BLOCK_HORIZON: u32 = 1;

// The size of a signature inside a transaction `Witness`
pub const WITNESS_STATIC_SIZE: usize = 8;
const SIGNATURE_SIZE: usize = 64;
pub const SIGNATURE_WITNESS_SIZE: usize = WITNESS_STATIC_SIZE + SIGNATURE_SIZE;
