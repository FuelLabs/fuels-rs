use fuel_tx::AssetId;

use crate::constants::{
    BASE_ASSET_ID, DEFAULT_CALL_PARAMS_AMOUNT, DEFAULT_GAS_LIMIT, DEFAULT_GAS_PRICE,
    DEFAULT_MATURITY,
};

#[derive(Debug, Copy, Clone)]
//ANCHOR: tx_parameter
pub struct TxParameters {
    pub gas_price: u64,
    pub gas_limit: u64,
    pub maturity: u64,
}
//ANCHOR_END: tx_parameter

#[derive(Debug)]
pub struct CallParameters {
    pub amount: u64,
    pub asset_id: AssetId,
    pub gas_forwarded: Option<u64>,
}

impl CallParameters {
    pub fn new(amount: Option<u64>, asset_id: Option<AssetId>, gas_forwarded: Option<u64>) -> Self {
        Self {
            amount: amount.unwrap_or(DEFAULT_CALL_PARAMS_AMOUNT),
            asset_id: asset_id.unwrap_or(BASE_ASSET_ID),
            gas_forwarded,
        }
    }
}

impl Default for CallParameters {
    fn default() -> Self {
        Self {
            amount: DEFAULT_CALL_PARAMS_AMOUNT,
            asset_id: BASE_ASSET_ID,
            gas_forwarded: None,
        }
    }
}

impl Default for TxParameters {
    fn default() -> Self {
        Self {
            gas_price: DEFAULT_GAS_PRICE,
            gas_limit: DEFAULT_GAS_LIMIT,
            // By default, transaction is immediately valid
            maturity: DEFAULT_MATURITY,
        }
    }
}

impl TxParameters {
    pub fn new(gas_price: Option<u64>, gas_limit: Option<u64>, maturity: Option<u64>) -> Self {
        Self {
            gas_price: gas_price.unwrap_or(DEFAULT_GAS_PRICE),
            gas_limit: gas_limit.unwrap_or(DEFAULT_GAS_LIMIT),
            maturity: maturity.unwrap_or(DEFAULT_MATURITY),
        }
    }
}
