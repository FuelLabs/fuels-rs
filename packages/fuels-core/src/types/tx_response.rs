use super::tx_status::Success;
use fuel_tx::TxId;

#[derive(Clone, Debug)]
pub struct TxResponse {
    pub tx_status: Success,
    pub tx_id: TxId,
}
