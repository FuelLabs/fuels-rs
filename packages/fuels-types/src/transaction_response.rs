use std::str::FromStr;

use chrono::DateTime;
use chrono::NaiveDateTime;
use chrono::Utc;
use fuel_gql_client::client::types::TransactionResponse as ClientTransactionResponse;
use fuel_gql_client::client::types::TransactionStatus as ClientTransactionStatus;
use fuel_tx::Bytes32;
use fuel_tx::Transaction;

#[derive(Debug, Clone)]
pub struct TransactionResponse {
    pub transaction: Transaction,
    pub status: TransactionStatus,
    pub block_id: Option<Bytes32>,
    pub time: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub enum TransactionStatus {
    Submitted(),
    Success(),
    Failure(),
    SqueezedOut(),
}

impl From<ClientTransactionResponse> for TransactionResponse {
    fn from(client_response: ClientTransactionResponse) -> Self {
        let block_id = match &client_response.status {
            ClientTransactionStatus::Submitted { .. }
            | ClientTransactionStatus::SqueezedOut { .. } => None,
            ClientTransactionStatus::Success { block_id, .. }
            | ClientTransactionStatus::Failure { block_id, .. } => Some(block_id),
        };
        let block_id = block_id.map(|id| {
            Bytes32::from_str(id).expect("Client returned block id with invalid format.")
        });

        let time = match &client_response.status {
            ClientTransactionStatus::Submitted { .. }
            | ClientTransactionStatus::SqueezedOut { .. } => None,
            ClientTransactionStatus::Success { time, .. }
            | ClientTransactionStatus::Failure { time, .. } => {
                let native = NaiveDateTime::from_timestamp_opt(time.to_unix(), 0);
                native.map(|time| DateTime::<Utc>::from_utc(time, Utc))
            }
        };

        Self {
            transaction: client_response.transaction,
            status: client_response.status.into(),
            block_id,
            time,
        }
    }
}

impl From<ClientTransactionStatus> for TransactionStatus {
    fn from(client_status: ClientTransactionStatus) -> Self {
        match client_status {
            ClientTransactionStatus::Submitted { .. } => TransactionStatus::Submitted(),
            ClientTransactionStatus::Success { .. } => TransactionStatus::Success(),
            ClientTransactionStatus::Failure { .. } => TransactionStatus::Failure(),
            ClientTransactionStatus::SqueezedOut { .. } => TransactionStatus::SqueezedOut(),
        }
    }
}
