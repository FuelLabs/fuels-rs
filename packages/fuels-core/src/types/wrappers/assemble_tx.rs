#![cfg(feature = "std")]

use fuel_core_client::client::types::assemble_tx::AssembleTransactionResult as FuelAssembleTxResult;

pub use fuel_core_client::client::types::assemble_tx::{
    Account, ChangePolicy, Predicate, RequiredBalance,
};

use crate::types::{transaction::TransactionType, tx_status::TxStatus};

#[derive(Debug, Clone)]
pub struct AssembleTransactionResult {
    pub transaction: TransactionType,
    pub status: TxStatus,
    pub gas_price: u64,
}

#[cfg(feature = "std")]
impl From<FuelAssembleTxResult> for AssembleTransactionResult {
    fn from(value: FuelAssembleTxResult) -> Self {
        Self {
            status: value.status.into(),
            transaction: value.transaction.into(),
            gas_price: value.gas_price,
        }
    }
}
