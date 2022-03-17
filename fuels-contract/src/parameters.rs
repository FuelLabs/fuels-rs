use crate::constants::{BYTE_PRICE, GAS_LIMIT, GAS_PRICE};

#[derive(Debug)]
pub struct TxParameters {
    pub gas_price: u64,
    pub gas_limit: u64,
    pub byte_price: u64,
}

impl Default for TxParameters {
    fn default() -> Self {
        Self {
            gas_price: GAS_PRICE,
            gas_limit: GAS_LIMIT,
            byte_price: BYTE_PRICE,
        }
    }
}

impl TxParameters {
    pub fn new(gas_price: Option<u64>, gas_limit: Option<u64>, byte_price: Option<u64>) -> Self {
        Self {
            gas_price: gas_price.unwrap_or(GAS_PRICE),
            gas_limit: gas_limit.unwrap_or(GAS_LIMIT),
            byte_price: byte_price.unwrap_or(BYTE_PRICE),
        }
    }
}
