#![cfg(feature = "std")]

use std::str::FromStr;

use chrono::{DateTime, NaiveDateTime, Utc};
use fuel_core_client::client::types::{
    TransactionResponse as ClientTransactionResponse, TransactionStatus as ClientTransactionStatus,
};
use fuel_tx::{ConsensusParameters, Transaction};
use fuel_types::Bytes32;

use crate::types::transaction::{CreateTransaction, ScriptTransaction, TransactionType};

#[derive(Debug, Clone)]
pub struct TransactionResponse {
    pub transaction: TransactionType,
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

impl TransactionResponse {
    pub fn from_fuel_response(
        client_response: ClientTransactionResponse,
        consensus_parameters: ConsensusParameters,
    ) -> Self {
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

        let transaction = match client_response.transaction {
            Transaction::Script(tx) => {
                TransactionType::Script(ScriptTransaction::from_fuel_tx(tx, consensus_parameters))
            }
            Transaction::Create(tx) => {
                TransactionType::Create(CreateTransaction::from_fuel_tx(tx, consensus_parameters))
            }
            Transaction::Mint(_) => unimplemented!(),
        };

        Self {
            transaction,
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
