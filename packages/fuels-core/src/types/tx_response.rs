use fuel_tx::{Receipt, TxId};

use crate::sealed::Sealed;

#[derive(Clone, Debug)]
pub struct TxResponse<T: TxResponseType = TxId> {
    pub receipts: Vec<Receipt>,
    pub gas_used: u64,
    pub total_fee: u64,
    pub id: T,
}

impl Sealed for TxId {}
impl Sealed for Option<TxId> {}

pub trait TxResponseType: Sealed {}

impl TxResponseType for TxId {}
impl TxResponseType for Option<TxId> {}
