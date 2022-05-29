use anyhow::Result;
use fuel_gql_client::{
    client::{types::TransactionStatus, FuelClient},
    fuel_tx::{Receipt, Transaction},
};
use fuels_core::errors::Error;

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

    // Calling the contract executes the transaction, and is thus state-modifying
    pub async fn call(self, fuel_client: &FuelClient) -> Result<Vec<Receipt>, Error> {
        let tx_id = fuel_client.submit(&self.tx).await?.0.to_string();
        let receipts = fuel_client.receipts(&tx_id).await?;
        let status = fuel_client.transaction_status(&tx_id).await?;
        match status {
            TransactionStatus::Failure { reason, .. } => {
                Err(Error::ContractCallError(reason, receipts))
            }
            _ => Ok(receipts),
        }
    }

    // Simulating a call to the contract means that the actual state of the blockchain is not
    // modified, it is only simulated using a "dry-run".
    pub async fn simulate(self, fuel_client: &FuelClient) -> Result<Vec<Receipt>, Error> {
        let receipts = fuel_client.dry_run(&self.tx).await?;
        Ok(receipts)
    }
}
