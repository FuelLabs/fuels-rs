use std::io;

#[cfg(feature = "fuel-core")]
use fuel_core::service::{Config, FuelService};

use fuel_gql_client::interpreter::ExecutableTransaction;
use fuel_gql_client::{
    client::{
        schema::{
            balance::Balance, chain::ChainInfo, coin::Coin, contract::ContractBalance,
            message::Message, node_info::NodeInfo, resource::Resource,
        },
        types::{TransactionResponse, TransactionStatus},
        FuelClient, PageDirection, PaginatedResult, PaginationRequest,
    },
    fuel_tx::{Receipt, Transaction, TransactionFee},
    fuel_types::AssetId,
};
use fuels_core::constants::{DEFAULT_GAS_ESTIMATION_TOLERANCE, MAX_GAS_PER_TX};
use std::collections::HashMap;
use thiserror::Error;

use crate::{field, UniqueIdentifier};
use fuels_types::bech32::{Bech32Address, Bech32ContractId};
use fuels_types::errors::Error;

#[derive(Debug)]
pub struct TransactionCost {
    pub min_gas_price: u64,
    pub gas_price: u64,
    pub gas_used: u64,
    pub metered_bytes_size: u64,
    pub total_fee: u64,
}

#[derive(Debug, Error)]
pub enum ProviderError {
    // Every IO error in the context of Provider comes from the gql client
    #[error(transparent)]
    ClientRequestError(#[from] io::Error),
}

impl From<ProviderError> for Error {
    fn from(e: ProviderError) -> Self {
        Error::ProviderError(e.to_string())
    }
}

/// Encapsulates common client operations in the SDK.
/// Note that you may also use `client`, which is an instance
/// of `FuelClient`, directly, which provides a broader API.
#[derive(Debug, Clone)]
pub struct Provider {
    pub client: FuelClient,
}

impl Provider {
    pub fn new(client: FuelClient) -> Self {
        Self { client }
    }

    /// Sends a transaction to the underlying Provider's client.
    /// # Examples
    ///
    /// ## Sending a transaction
    ///
    /// ```
    /// use fuels::tx::Script;
    /// use fuels::prelude::*;
    /// async fn foo() -> Result<(), Box<dyn std::error::Error>> {
    ///   // Setup local test node
    ///   let (provider, _) = setup_test_provider(vec![], vec![], None, None).await;
    ///   let tx = Script::default();
    ///
    ///   let receipts = provider.send_transaction(&tx).await?;
    ///   dbg!(receipts);
    ///
    ///   Ok(())
    /// }
    /// ```
    pub async fn send_transaction<Tx>(&self, tx: &Tx) -> Result<Vec<Receipt>, Error>
    where
        Tx: ExecutableTransaction + field::GasLimit + field::GasPrice + Into<Transaction>,
    {
        let tolerance = 0.0;
        let TransactionCost {
            gas_used,
            min_gas_price,
            ..
        } = self.estimate_transaction_cost(tx, Some(tolerance)).await?;

        if gas_used > *tx.gas_limit() {
            return Err(Error::ProviderError(format!(
                "gas_limit({}) is lower than the estimated gas_used({})",
                tx.gas_limit(),
                gas_used
            )));
        } else if min_gas_price > *tx.gas_price() {
            return Err(Error::ProviderError(format!(
                "gas_price({}) is lower than the required min_gas_price({})",
                tx.gas_price(),
                min_gas_price
            )));
        }

        let (status, receipts) = self.submit_with_feedback(&tx.clone().into()).await?;

        if let TransactionStatus::Failure { reason, .. } = status {
            Err(Error::RevertTransactionError(reason, receipts))
        } else {
            Ok(receipts)
        }
    }

    async fn submit_with_feedback(
        &self,
        tx: &Transaction,
    ) -> Result<(TransactionStatus, Vec<Receipt>), ProviderError> {
        let tx_id = tx.id().to_string();
        let status = self.client.submit_and_await_commit(tx).await?;
        let receipts = self.client.receipts(&tx_id).await?;

        Ok((status, receipts))
    }

    #[cfg(feature = "fuel-core")]
    /// Launches a local `fuel-core` network based on provided config.
    pub async fn launch(config: Config) -> Result<FuelClient, Error> {
        let srv = FuelService::new_node(config).await.unwrap();
        Ok(FuelClient::from(srv.bound_address))
    }

    /// Connects to an existing node at the given address.
    /// # Examples
    ///
    /// ## Connect to a node
    /// ```
    /// async fn connect_to_fuel_node() {
    ///     use fuels::prelude::*;
    ///
    ///     // This is the address of a running node.
    ///     let server_address = "127.0.0.1:4000";
    ///
    ///     // Create the provider using the client.
    ///     let provider = Provider::connect(server_address).await.unwrap();
    ///
    ///     // Create the wallet.
    ///     let _wallet = WalletUnlocked::new_random(Some(provider));
    /// }
    /// ```
    pub async fn connect(url: impl AsRef<str>) -> Result<Provider, Error> {
        let client = FuelClient::new(url)?;
        Ok(Provider::new(client))
    }

    pub async fn chain_info(&self) -> Result<ChainInfo, ProviderError> {
        Ok(self.client.chain_info().await?)
    }

    pub async fn node_info(&self) -> Result<NodeInfo, ProviderError> {
        Ok(self.client.node_info().await?)
    }

    pub async fn dry_run(&self, tx: &Transaction) -> Result<Vec<Receipt>, ProviderError> {
        Ok(self.client.dry_run(tx).await?)
    }

    pub async fn dry_run_no_validation(
        &self,
        tx: &Transaction,
    ) -> Result<Vec<Receipt>, ProviderError> {
        Ok(self.client.dry_run_opt(tx, Some(false)).await?)
    }

    /// Gets all coins owned by address `from`, with asset ID `asset_id`, *even spent ones*. This
    /// returns actual coins (UTXOs).
    pub async fn get_coins(
        &self,
        from: &Bech32Address,
        asset_id: AssetId,
    ) -> Result<Vec<Coin>, ProviderError> {
        let mut coins: Vec<Coin> = vec![];

        let mut cursor = None;

        loop {
            let res = self
                .client
                .coins(
                    &from.hash().to_string(),
                    Some(&asset_id.to_string()),
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
    pub async fn get_spendable_resources(
        &self,
        from: &Bech32Address,
        asset_id: AssetId,
        amount: u64,
    ) -> Result<Vec<Resource>, ProviderError> {
        let res = self
            .client
            .resources_to_spend(
                &from.hash().to_string(),
                vec![(format!("{:#x}", asset_id).as_str(), amount, None)],
                None,
            )
            .await?
            .into_iter()
            .flatten()
            .collect();

        Ok(res)
    }

    /// Get the balance of all spendable coins `asset_id` for address `address`. This is different
    /// from getting coins because we are just returning a number (the sum of UTXOs amount) instead
    /// of the UTXOs.
    pub async fn get_asset_balance(
        &self,
        address: &Bech32Address,
        asset_id: AssetId,
    ) -> Result<u64, ProviderError> {
        self.client
            .balance(&address.hash().to_string(), Some(&*asset_id.to_string()))
            .await
            .map_err(Into::into)
    }

    /// Get the balance of all spendable coins `asset_id` for contract with id `contract_id`.
    pub async fn get_contract_asset_balance(
        &self,
        contract_id: &Bech32ContractId,
        asset_id: AssetId,
    ) -> Result<u64, ProviderError> {
        self.client
            .contract_balance(&contract_id.hash().to_string(), Some(&asset_id.to_string()))
            .await
            .map_err(Into::into)
    }

    /// Get all the spendable balances of all assets for address `address`. This is different from
    /// getting the coins because we are only returning the numbers (the sum of UTXOs coins amount
    /// for each asset id) and not the UTXOs coins themselves
    pub async fn get_balances(
        &self,
        address: &Bech32Address,
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
            .balances(&address.hash().to_string(), pagination)
            .await?
            .results;
        let balances = balances_vec
            .into_iter()
            .map(
                |Balance {
                     owner: _,
                     amount,
                     asset_id,
                 }| (asset_id.to_string(), amount.try_into().unwrap()),
            )
            .collect();
        Ok(balances)
    }

    /// Get all balances of all assets for the contract with id `contract_id`.
    pub async fn get_contract_balances(
        &self,
        contract_id: &Bech32ContractId,
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
            .contract_balances(&contract_id.hash().to_string(), pagination)
            .await?
            .results;
        let balances = balances_vec
            .into_iter()
            .map(
                |ContractBalance {
                     contract: _,
                     amount,
                     asset_id,
                 }| (asset_id.to_string(), amount.try_into().unwrap()),
            )
            .collect();
        Ok(balances)
    }

    /// Get transaction by id.
    pub async fn get_transaction_by_id(
        &self,
        tx_id: &str,
    ) -> Result<TransactionResponse, ProviderError> {
        Ok(self.client.transaction(tx_id).await.unwrap().unwrap())
    }

    // - Get transaction(s)
    pub async fn get_transactions(
        &self,
        request: PaginationRequest<String>,
    ) -> Result<PaginatedResult<TransactionResponse, String>, ProviderError> {
        self.client.transactions(request).await.map_err(Into::into)
    }

    // Get transaction(s) by owner
    pub async fn get_transactions_by_owner(
        &self,
        owner: &Bech32Address,
        request: PaginationRequest<String>,
    ) -> Result<PaginatedResult<TransactionResponse, String>, ProviderError> {
        self.client
            .transactions_by_owner(&owner.hash().to_string(), request)
            .await
            .map_err(Into::into)
    }

    pub async fn latest_block_height(&self) -> Result<u64, ProviderError> {
        Ok(self.client.chain_info().await?.latest_block.header.height.0)
    }

    pub async fn produce_blocks(&self, amount: u64) -> io::Result<u64> {
        self.client.produce_blocks(amount, None).await
    }

    pub async fn estimate_transaction_cost<Tx>(
        &self,
        tx: &Tx,
        tolerance: Option<f64>,
    ) -> Result<TransactionCost, Error>
    where
        Tx: ExecutableTransaction + field::GasLimit + field::GasPrice,
    {
        let NodeInfo { min_gas_price, .. } = self.node_info().await?;

        let tolerance = tolerance.unwrap_or(DEFAULT_GAS_ESTIMATION_TOLERANCE);
        let mut dry_run_tx = Self::generate_dry_run_tx(tx);
        let consensus_parameters = self.chain_info().await?.consensus_parameters;
        let gas_used = self
            .get_gas_used_with_tolerance(&dry_run_tx, tolerance)
            .await?;
        let gas_price = std::cmp::max(*tx.gas_price(), min_gas_price.0);

        // Update the dry_run_tx with estimated gas_used and correct gas price to calculate the total_fee
        *dry_run_tx.gas_price_mut() = gas_price;
        *dry_run_tx.gas_limit_mut() = gas_used;

        let transaction_fee =
            TransactionFee::checked_from_tx(&consensus_parameters.into(), &dry_run_tx)
                .expect("Error calculating TransactionFee");

        Ok(TransactionCost {
            min_gas_price: min_gas_price.0,
            gas_price,
            gas_used,
            metered_bytes_size: tx.metered_bytes_size() as u64,
            total_fee: transaction_fee.total(),
        })
    }

    // Remove limits from an existing Transaction to get an accurate gas estimation
    fn generate_dry_run_tx<Tx: field::GasPrice + field::GasLimit + Clone>(tx: &Tx) -> Tx {
        let mut dry_run_tx = tx.clone();
        // Simulate the contract call with MAX_GAS_PER_TX to get the complete gas_used
        *dry_run_tx.gas_limit_mut() = MAX_GAS_PER_TX;
        *dry_run_tx.gas_price_mut() = 0;
        dry_run_tx
    }

    // Increase estimated gas by the provided tolerance
    async fn get_gas_used_with_tolerance<Tx: Into<Transaction> + Clone>(
        &self,
        tx: &Tx,
        tolerance: f64,
    ) -> Result<u64, ProviderError> {
        let gas_used = self.get_gas_used(&self.dry_run_no_validation(&tx.clone().into()).await?);
        Ok((gas_used as f64 * (1.0 + tolerance)) as u64)
    }

    fn get_gas_used(&self, receipts: &[Receipt]) -> u64 {
        receipts
            .iter()
            .rfind(|r| matches!(r, Receipt::ScriptResult { .. }))
            .map(|script_result| {
                script_result
                    .gas_used()
                    .expect("could not retrieve gas used from ScriptResult")
            })
            .unwrap_or(0)
    }

    pub async fn get_messages(&self, from: &Bech32Address) -> Result<Vec<Message>, ProviderError> {
        let pagination = PaginationRequest {
            cursor: None,
            results: 100,
            direction: PageDirection::Forward,
        };
        let res = self
            .client
            .messages(Some(&from.hash().to_string()), pagination)
            .await?;
        Ok(res.results)
    }
}
