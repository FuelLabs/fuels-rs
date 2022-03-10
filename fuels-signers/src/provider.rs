use fuel_core::service::{Config, FuelService};
use fuel_gql_client::client::schema::coin::Coin;
use fuel_gql_client::client::{FuelClient, PageDirection, PaginationRequest};
use fuel_tx::Receipt;
use fuel_tx::{Address, AssetId, Input, Output, Transaction};
use fuel_vm::consts::REG_ONE;
use std::io;
use std::net::SocketAddr;

use fuel_vm::prelude::Opcode;
use fuels_core::errors::Error;
use thiserror::Error;

/// An error involving a signature.
#[derive(Debug, Error)]
pub enum ProviderError {
    #[error("Request failed: {0}")]
    TransactionRequestError(String),
    #[error(transparent)]
    ClientRequestError(#[from] io::Error),
}

/// Encapsulates common client operations in the SDK.
/// Note that you may also use `client`, which is an instance
/// of `FuelClient`, directly, which providers a broader API.
#[derive(Debug, Clone)]
pub struct Provider {
    pub client: FuelClient,
}

impl Provider {
    pub fn new(client: FuelClient) -> Self {
        Self { client }
    }

    /// Shallow wrapper on client's submit.
    pub async fn send_transaction(&self, tx: &Transaction) -> io::Result<Vec<Receipt>> {
        let tx_id = self.client.submit(tx).await?;

        Ok(self.client.receipts(&tx_id.0.to_string()).await?)
    }

    /// Launches a local `fuel-core` network based on provided config.
    pub async fn launch(config: Config) -> Result<FuelClient, Error> {
        let srv = FuelService::new_node(config).await.unwrap();
        Ok(FuelClient::from(srv.bound_address))
    }

    /// Connects to an existing node at the given address
    pub async fn connect(socket: SocketAddr) -> Result<FuelClient, Error> {
        Ok(FuelClient::from(socket))
    }

    /// Shallow wrapper on client's coins API.
    pub async fn get_coins(&self, from: &Address) -> Result<Vec<Coin>, ProviderError> {
        let mut coins: Vec<Coin> = vec![];

        let mut cursor = None;

        loop {
            let res = self
                .client
                .coins(
                    &from.to_string(),
                    None,
                    PaginationRequest {
                        cursor: cursor.clone(),
                        results: 100,
                        direction: PageDirection::Forward,
                    },
                )
                .await?;

            if res.results.is_empty() {
                break;
            }
            coins.extend(res.results);
            cursor = res.cursor;
        }

        Ok(coins)
    }

    pub async fn get_spendable_coins(
        &self,
        from: &Address,
        asset_id: AssetId,
        amount: u64,
    ) -> io::Result<Vec<Coin>> {
        let res = self
            .client
            .coins_to_spend(
                &from.to_string(),
                vec![(format!("{:#x}", asset_id).as_str(), amount)],
                None,
            )
            .await?;

        Ok(res)
    }

    /// Craft a transaction used to transfer funds between two addresses.
    pub fn build_transfer_tx(&self, inputs: &[Input], outputs: &[Output]) -> Transaction {
        // This script contains a single Opcode that returns immediately (RET)
        // since all this transaction does is move Inputs and Outputs around.
        let script = Opcode::RET(REG_ONE).to_bytes().to_vec();
        Transaction::Script {
            gas_price: 0,
            gas_limit: 1_000_000,
            byte_price: 0,
            maturity: 0,
            receipts_root: Default::default(),
            script,
            script_data: vec![],
            inputs: inputs.to_vec(),
            outputs: outputs.to_vec(),
            witnesses: vec![],
            metadata: None,
        }
    }

    // @todo
    // - Get transaction(s)
    // - Get block(s)
}
