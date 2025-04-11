use fuel_tx::TxId;

use super::tx_status::Success;

#[derive(Clone, Debug)]
pub struct TxResponse {
    pub tx_status: Success,
    pub tx_id: TxId,
}
