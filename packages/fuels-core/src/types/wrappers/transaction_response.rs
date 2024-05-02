#![cfg(feature = "std")]

use chrono::{DateTime, Utc};
use fuel_core_client::client::types::{
    TransactionResponse as ClientTransactionResponse, TransactionStatus as ClientTransactionStatus,
};
use fuel_tx::Transaction;
use fuel_types::BlockHeight;

use crate::types::{
    transaction::{CreateTransaction, ScriptTransaction, TransactionType},
    tx_status::TxStatus,
};

#[derive(Debug, Clone)]
pub struct TransactionResponse {
    pub transaction: TransactionType,
    pub status: TxStatus,
    pub block_height: Option<BlockHeight>,
    pub time: Option<DateTime<Utc>>,
}

impl From<ClientTransactionResponse> for TransactionResponse {
    fn from(client_response: ClientTransactionResponse) -> Self {
        let block_height = match &client_response.status {
            ClientTransactionStatus::Submitted { .. }
            | ClientTransactionStatus::SqueezedOut { .. } => None,
            ClientTransactionStatus::Success { block_height, .. }
            | ClientTransactionStatus::Failure { block_height, .. } => Some(*block_height),
        };

        let time = match &client_response.status {
            ClientTransactionStatus::Submitted { .. }
            | ClientTransactionStatus::SqueezedOut { .. } => None,
            ClientTransactionStatus::Success { time, .. }
            | ClientTransactionStatus::Failure { time, .. } => {
                DateTime::from_timestamp(time.to_unix(), 0)
            }
        };

        let transaction = match client_response.transaction {
            Transaction::Script(tx) => TransactionType::Script(ScriptTransaction::from(tx)),
            Transaction::Create(tx) => TransactionType::Create(CreateTransaction::from(tx)),
            Transaction::Mint(tx) => TransactionType::Mint(tx.into()),
            Transaction::Upgrade(tx) => TransactionType::Upgrade(tx.into()),
            Transaction::Upload(tx) => TransactionType::Upload(tx.into()),
        };

        Self {
            transaction,
            status: client_response.status.into(),
            block_height,
            time,
        }
    }
}
