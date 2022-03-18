use fuels_core::constants::{DEFAULT_BYTE_PRICE, DEFAULT_GAS_LIMIT, DEFAULT_GAS_PRICE};

#[derive(Debug)]
pub struct TxParameters {
    pub gas_price: u64,
    pub gas_limit: u64,
    pub byte_price: u64,
}

impl Default for TxParameters {
    fn default() -> Self {
        Self {
            gas_price: DEFAULT_GAS_PRICE,
            gas_limit: DEFAULT_GAS_LIMIT,
            byte_price: DEFAULT_BYTE_PRICE,
        }
    }
}

impl TxParameters {
    pub fn new(gas_price: Option<u64>, gas_limit: Option<u64>, byte_price: Option<u64>) -> Self {
        Self {
            gas_price: gas_price.unwrap_or(DEFAULT_GAS_PRICE),
            gas_limit: gas_limit.unwrap_or(DEFAULT_GAS_LIMIT),
            byte_price: byte_price.unwrap_or(DEFAULT_BYTE_PRICE),
        }
    }
}
