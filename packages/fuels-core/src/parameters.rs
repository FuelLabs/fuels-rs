use crate::constants::{
    DEFAULT_BYTE_PRICE, DEFAULT_GAS_LIMIT, DEFAULT_GAS_PRICE, DEFAULT_MATURITY, NATIVE_ASSET_ID,
};
use fuel_tx::AssetId;

#[derive(Debug)]
pub struct TxParameters {
    pub gas_price: u64,
    pub gas_limit: u64,
    pub byte_price: u64,
    pub maturity: u64,
}

#[derive(Debug)]
pub struct CallParameters {
    pub amount: u64,
    pub asset_id: AssetId,
}

impl CallParameters {
    pub fn new(amount: Option<u64>, asset_id: Option<AssetId>) -> Self {
        Self {
            amount: amount.unwrap_or(0),
            asset_id: asset_id.unwrap_or(NATIVE_ASSET_ID),
        }
    }
}

impl Default for CallParameters {
    fn default() -> Self {
        Self {
            amount: 0,
            asset_id: NATIVE_ASSET_ID,
        }
    }
}

impl Default for TxParameters {
    fn default() -> Self {
        Self {
            gas_price: DEFAULT_GAS_PRICE,
            gas_limit: DEFAULT_GAS_LIMIT,
            byte_price: DEFAULT_BYTE_PRICE,
            // By default, transaction is immediately valid
            maturity: DEFAULT_MATURITY,
        }
    }
}

impl TxParameters {
    pub fn new(
        gas_price: Option<u64>,
        gas_limit: Option<u64>,
        byte_price: Option<u64>,
        maturity: Option<u64>,
    ) -> Self {
        Self {
            gas_price: gas_price.unwrap_or(DEFAULT_GAS_PRICE),
            gas_limit: gas_limit.unwrap_or(DEFAULT_GAS_LIMIT),
            byte_price: byte_price.unwrap_or(DEFAULT_BYTE_PRICE),
            maturity: maturity.unwrap_or(DEFAULT_MATURITY),
        }
    }
}
