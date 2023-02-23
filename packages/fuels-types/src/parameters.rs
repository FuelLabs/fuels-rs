use fuel_tx::AssetId;

use crate::constants::{BASE_ASSET_ID, DEFAULT_CALL_PARAMS_AMOUNT};

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

