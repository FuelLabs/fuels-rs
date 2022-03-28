use crate::errors::Error;
use anyhow::Result;
use fuel_gql_client::client::{types::TransactionStatus, FuelClient};
use fuel_tx::{Receipt, Transaction};

/// Script is a very thin layer on top of fuel-client with some
/// extra functionalities needed and provided by the SDK.
pub struct Script {
    pub tx: Transaction,
}

#[derive(Debug, Clone)]
pub struct CompiledScript {
    pub raw: Vec<u8>,
    pub target_network_url: String,
}

impl Script {
    pub fn new(tx: Transaction) -> Self {
        Self { tx }
    }

    // Sending a transactions means spending tokens to execute the transaction. It is
    // generally meant for executing state-modifying transactions.
    pub async fn send(self, fuel_client: &FuelClient) -> Result<Vec<Receipt>, Error> {
        let tx_id = fuel_client.submit(&self.tx).await?.0.to_string();

        let receipts = fuel_client.receipts(&tx_id).await?;
        let status = fuel_client.transaction_status(&tx_id).await?;
        match status {
            TransactionStatus::Failure { reason, .. } => Err(Error::ContractCallError(reason)),
            _ => Ok(receipts),
        }
    }

    // Calling a contract means that the state of the contract is not modified, this can
    // be seen as being a "read-only" transaction on the state of the contract
    pub async fn call(self, fuel_client: &FuelClient) -> Result<Vec<Receipt>, Error> {
        let receipts = fuel_client.dry_run(&self.tx).await?;
        Ok(receipts)
    }
}
