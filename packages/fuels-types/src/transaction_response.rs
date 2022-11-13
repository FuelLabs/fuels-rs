use fuel_gql_client::client::types::TransactionResponse as SchemaTransactionResponse;
use fuel_gql_client::client::types::TransactionStatus as SchemaTransactionStatus;
use fuel_tx::Transaction;

#[derive(Debug, Clone)]
pub struct TransactionResponse {
    schema_response: SchemaTransactionResponse,
}

#[derive(Debug, Clone)]
pub enum TransactionStatus {
    Submitted(),
    Success(),
    Failure(),
    SqueezedOut(),
}

impl From<SchemaTransactionResponse> for TransactionResponse {
    fn from(schema_response: SchemaTransactionResponse) -> Self {
        Self { schema_response }
    }
}

impl From<&SchemaTransactionStatus> for TransactionStatus {
    fn from(schema_status: &SchemaTransactionStatus) -> Self {
        match schema_status {
            SchemaTransactionStatus::Submitted { .. } => TransactionStatus::Submitted(),
            SchemaTransactionStatus::Success { .. } => TransactionStatus::Success(),
            SchemaTransactionStatus::Failure { .. } => TransactionStatus::Failure(),
            SchemaTransactionStatus::SqueezedOut { .. } => TransactionStatus::SqueezedOut(),
        }
    }
}

impl TransactionResponse {
    pub fn status(&self) -> TransactionStatus {
        (&self.schema_response.status).into()
    }

    pub fn transaction(&self) -> &Transaction {
        &self.schema_response.transaction
    }

    pub fn block_id(&self) -> Option<&str> {
        match &self.schema_response.status {
            SchemaTransactionStatus::Submitted { .. }
            | SchemaTransactionStatus::SqueezedOut { .. } => None,
            SchemaTransactionStatus::Success { block_id, .. }
            | SchemaTransactionStatus::Failure { block_id, .. } => Some(block_id),
        }
    }

    pub fn time(&self) -> Option<u64> {
        match &self.schema_response.status {
            SchemaTransactionStatus::Submitted { .. }
            | SchemaTransactionStatus::SqueezedOut { .. } => None,
            SchemaTransactionStatus::Success { time, .. }
            | SchemaTransactionStatus::Failure { time, .. } => Some(time.0),
        }
    }
}
