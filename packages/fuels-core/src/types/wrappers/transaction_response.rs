#![cfg(feature = "std")]

use chrono::{DateTime, Utc};
use fuel_core_client::client::types::{
    TransactionResponse as ClientTransactionResponse, TransactionStatus as ClientTransactionStatus,
    TransactionType as ClientTxType,
};
use fuel_tx::Transaction;
use fuel_types::BlockHeight;

use crate::types::{transaction::TransactionType, tx_status::TxStatus};

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
            | ClientTransactionStatus::SqueezedOut { .. }
            | ClientTransactionStatus::PreconfirmationSuccess { .. }
            | ClientTransactionStatus::PreconfirmationFailure { .. } => None,
            ClientTransactionStatus::Success { block_height, .. }
            | ClientTransactionStatus::Failure { block_height, .. } => Some(*block_height),
        };

        let time = match &client_response.status {
            ClientTransactionStatus::Submitted { .. }
            | ClientTransactionStatus::SqueezedOut { .. }
            | ClientTransactionStatus::PreconfirmationSuccess { .. }
            | ClientTransactionStatus::PreconfirmationFailure { .. } => None,
            ClientTransactionStatus::Success { time, .. }
            | ClientTransactionStatus::Failure { time, .. } => {
                DateTime::from_timestamp(time.to_unix(), 0)
            }
        };

        let transaction = match client_response.transaction {
            ClientTxType::Known(Transaction::Script(tx)) => TransactionType::Script(tx.into()),
            ClientTxType::Known(Transaction::Create(tx)) => TransactionType::Create(tx.into()),
            ClientTxType::Known(Transaction::Mint(tx)) => TransactionType::Mint(tx.into()),
            ClientTxType::Known(Transaction::Upgrade(tx)) => TransactionType::Upgrade(tx.into()),
            ClientTxType::Known(Transaction::Upload(tx)) => TransactionType::Upload(tx.into()),
            ClientTxType::Known(Transaction::Blob(tx)) => TransactionType::Blob(tx.into()),
            ClientTxType::Unknown => TransactionType::Unknown,
        };

        Self {
            transaction,
            status: client_response.status.into(),
            block_height,
            time,
        }
    }
}
