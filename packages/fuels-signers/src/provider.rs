use std::io;
use std::net::SocketAddr;

#[cfg(feature = "fuel-core")]
use fuel_core::service::{Config, FuelService};
use fuel_gql_client::{
    client::{
        schema::coin::Coin, types::TransactionResponse, FuelClient, PageDirection, PaginatedResult,
        PaginationRequest,
    },
    fuel_tx::{Input, Output, Receipt, Transaction},
    fuel_types::{Address, AssetId},
    fuel_vm::{consts::REG_ONE, prelude::Opcode},
};
use std::collections::HashMap;
use thiserror::Error;

use crate::wallet::WalletError;
use fuels_core::parameters::TxParameters;
use fuels_types::errors::Error;

/// An error involving a signature.
#[derive(Debug, Error)]
pub enum ProviderError {
    #[error("Request failed: {0}")]
    TransactionRequestError(String),
    #[error(transparent)]
    ClientRequestError(#[from] io::Error),
    #[error("Wallet error: {0}")]
    WalletError(String),
}

impl From<WalletError> for ProviderError {
    fn from(e: WalletError) -> Self {
        ProviderError::WalletError(e.to_string())
    }
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

        self.client.receipts(&tx_id.0.to_string()).await
    }

    #[cfg(feature = "fuel-core")]
    /// Launches a local `fuel-core` network based on provided config.
    pub async fn launch(config: Config) -> Result<FuelClient, Error> {
        let srv = FuelService::new_node(config).await.unwrap();
        Ok(FuelClient::from(srv.bound_address))
    }

    /// Connects to an existing node at the given address
    pub async fn connect(socket: SocketAddr) -> Result<Provider, Error> {
        Ok(Self {
            client: FuelClient::from(socket),
        })
    }

    /// Gets all coins owned by address `from`, *even spent ones*. This returns actual coins
    /// (UTXOs).
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

    /// Get some spendable coins of asset `asset_id` for address `from` that add up at least to
    /// amount `amount`. The returned coins (UTXOs) are actual coins that can be spent. The number
    /// of coins (UXTOs) is optimized to prevent dust accumulation.
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
                None,
            )
            .await?;
        Ok(res)
    }

    /// Craft a transaction used to transfer funds between two addresses.
    pub fn build_transfer_tx(
        &self,
        inputs: &[Input],
        outputs: &[Output],
        params: TxParameters,
    ) -> Transaction {
        // This script contains a single Opcode that returns immediately (RET)
        // since all this transaction does is move Inputs and Outputs around.
        let script = Opcode::RET(REG_ONE).to_bytes().to_vec();
        Transaction::Script {
            gas_price: params.gas_price,
            gas_limit: params.gas_limit,
            byte_price: params.byte_price,
            maturity: params.maturity,
            receipts_root: Default::default(),
            script,
            script_data: vec![],
            inputs: inputs.to_vec(),
            outputs: outputs.to_vec(),
            witnesses: vec![],
            metadata: None,
        }
    }
    // TODO: add unit tests for the balance API. This is tracked in #321.

    /// Get the balance of all spendable coins `asset_id` for address `address`. This is different
    /// from getting coins because we are just returning a number (the sum of UTXOs amount) instead
    /// of the UTXOs.
    pub async fn get_asset_balance(
        &self,
        address: &Address,
        asset_id: AssetId,
    ) -> Result<u64, ProviderError> {
        self.client
            .balance(&*address.to_string(), Some(&*asset_id.to_string()))
            .await
            .map_err(Into::into)
    }

    /// Get all the spendable balances of all assets for address `address`. This is different from
    /// getting the coins because we are only returning the numbers (the sum of UTXOs coins amount
    /// for each asset id) and not the UTXOs coins themselves
    pub async fn get_balances(
        &self,
        address: &Address,
    ) -> Result<HashMap<String, u64>, ProviderError> {
        // We don't paginate results because there are likely at most ~100 different assets in one
        // wallet
        let pagination = PaginationRequest {
            cursor: None,
            results: 9999,
            direction: PageDirection::Forward,
        };
        let balances_vec = self
            .client
            .balances(&*address.to_string(), pagination)
            .await?
            .results;
        let balances = balances_vec
            .iter()
            .map(|b| (b.asset_id.to_string(), b.amount.clone().try_into().unwrap()))
            .collect();
        Ok(balances)
    }

    /// Get transaction by id.
    pub async fn get_transaction_by_id(&self, tx_id: &str) -> io::Result<TransactionResponse> {
        Ok(self.client.transaction(tx_id).await.unwrap().unwrap())
    }

    // - Get transaction(s)
    pub async fn get_transactions(
        &self,
        request: PaginationRequest<String>,
    ) -> std::io::Result<PaginatedResult<TransactionResponse, String>> {
        self.client.transactions(request).await
    }

    // - Get transaction(s) by owner
    pub async fn get_transactions_by_owner(
        &self,
        owner: &str,
        request: PaginationRequest<String>,
    ) -> std::io::Result<PaginatedResult<TransactionResponse, String>> {
        self.client.transactions_by_owner(owner, request).await
    }

    pub async fn latest_block_height(&self) -> io::Result<u64> {
        Ok(self.client.chain_info().await?.latest_block.height.0)
    }

    // @todo
    // - Get block(s)
}
