use std::{collections::HashMap, io};

use chrono::{DateTime, Duration, Utc};
#[cfg(feature = "fuel-core")]
use fuel_core::service::{Config, FuelService};
use fuel_core_client::client::{
    schema::{
        balance::Balance, block::TimeParameters as FuelTimeParameters, contract::ContractBalance,
    },
    types::TransactionStatus,
    FuelClient, PageDirection, PaginatedResult, PaginationRequest,
};
use fuel_tx::{AssetId, ConsensusParameters, Receipt};
use fuel_vm::state::ProgramState;
use fuels_types::{
    bech32::{Bech32Address, Bech32ContractId},
    block::Block,
    chain_info::ChainInfo,
    coin::Coin,
    constants::{DEFAULT_GAS_ESTIMATION_TOLERANCE, MAX_GAS_PER_TX},
    errors::{error, Error, Result},
    message::Message,
    message_proof::MessageProof,
    node_info::NodeInfo,
    resource::Resource,
    transaction::Transaction,
    transaction_response::TransactionResponse,
};
use tai64::Tai64;
use thiserror::Error;

type ProviderResult<T> = std::result::Result<T, ProviderError>;

#[derive(Debug)]
pub struct TransactionCost {
    pub min_gas_price: u64,
    pub gas_price: u64,
    pub gas_used: u64,
    pub metered_bytes_size: u64,
    pub total_fee: u64,
}

#[derive(Debug)]
// ANCHOR: time_parameters
pub struct TimeParameters {
    // The time to set on the first block
    pub start_time: DateTime<Utc>,
    // The time interval between subsequent blocks
    pub block_time_interval: Duration,
}
// ANCHOR_END: time_parameters

impl From<TimeParameters> for FuelTimeParameters {
    fn from(time: TimeParameters) -> Self {
        Self {
            start_time: Tai64::from_unix(time.start_time.timestamp()).0.into(),
            block_time_interval: (time.block_time_interval.num_seconds() as u64).into(),
        }
    }
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
    /// use fuels::prelude::*;
    /// async fn foo() -> std::result::Result<(), Box<dyn std::error::Error>> {
    ///   // Setup local test node
    ///   let (provider, _) = setup_test_provider(vec![], vec![], None, None).await;
    ///   let tx = ScriptTransaction::default();
    ///
    ///   let receipts = provider.send_transaction(&tx).await?;
    ///   dbg!(receipts);
    ///
    ///   Ok(())
    /// }
    /// ```
    pub async fn send_transaction<T: Transaction + Clone>(&self, tx: &T) -> Result<Vec<Receipt>> {
        let tolerance = 0.0;
        let TransactionCost {
            gas_used,
            min_gas_price,
            ..
        } = self.estimate_transaction_cost(tx, Some(tolerance)).await?;

        if gas_used > tx.gas_limit() {
            return Err(error!(
                ProviderError,
                "gas_limit({}) is lower than the estimated gas_used({})",
                tx.gas_limit(),
                gas_used
            ));
        } else if min_gas_price > tx.gas_price() {
            return Err(error!(
                ProviderError,
                "gas_price({}) is lower than the required min_gas_price({})",
                tx.gas_price(),
                min_gas_price
            ));
        }

        let chain_info = self.chain_info().await?;
        tx.check_without_signatures(
            chain_info.latest_block.header.height,
            &chain_info.consensus_parameters,
        )?;

        let (status, receipts) = self.submit_with_feedback(tx.clone()).await?;
        Self::if_failure_generate_error(&status, &receipts)?;

        Ok(receipts)
    }

    fn if_failure_generate_error(status: &TransactionStatus, receipts: &[Receipt]) -> Result<()> {
        if let TransactionStatus::Failure {
            reason,
            program_state,
            ..
        } = status
        {
            let revert_id = program_state
                .and_then(|state| match state {
                    ProgramState::Revert(revert_id) => Some(revert_id),
                    _ => None,
                })
                .expect("Transaction failed without a `revert_id`");

            return Err(Error::RevertTransactionError {
                reason: reason.to_string(),
                revert_id,
                receipts: receipts.to_owned(),
            });
        }

        Ok(())
    }

    async fn submit_with_feedback(
        &self,
        tx: impl Transaction,
    ) -> ProviderResult<(TransactionStatus, Vec<Receipt>)> {
        let tx_id = tx.id().to_string();
        let status = self.client.submit_and_await_commit(&tx.into()).await?;
        let receipts = self.client.receipts(&tx_id).await?;

        Ok((status, receipts))
    }

    #[cfg(feature = "fuel-core")]
    /// Launches a local `fuel-core` network based on provided config.
    pub async fn launch(config: Config) -> Result<FuelClient> {
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
    pub async fn connect(url: impl AsRef<str>) -> Result<Provider> {
        let client = FuelClient::new(url).map_err(|err| error!(InfrastructureError, "{err}"))?;
        Ok(Provider::new(client))
    }

    pub async fn chain_info(&self) -> ProviderResult<ChainInfo> {
        Ok(self.client.chain_info().await?.into())
    }

    pub async fn consensus_parameters(&self) -> ProviderResult<ConsensusParameters> {
        Ok(self.client.chain_info().await?.consensus_parameters.into())
    }

    pub async fn node_info(&self) -> ProviderResult<NodeInfo> {
        Ok(self.client.node_info().await?.into())
    }

    pub async fn dry_run<T: Transaction + Clone>(&self, tx: &T) -> Result<Vec<Receipt>> {
        let receipts = self.client.dry_run(&tx.clone().into()).await?;

        Ok(receipts)
    }

    pub async fn dry_run_no_validation<T: Transaction + Clone>(
        &self,
        tx: &T,
    ) -> Result<Vec<Receipt>> {
        let receipts = self
            .client
            .dry_run_opt(&tx.clone().into(), Some(false))
            .await?;

        Ok(receipts)
    }

    /// Gets all coins owned by address `from`, with asset ID `asset_id`, *even spent ones*. This
    /// returns actual coins (UTXOs).
    pub async fn get_coins(
        &self,
        from: &Bech32Address,
        asset_id: AssetId,
    ) -> ProviderResult<Vec<Coin>> {
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
            coins.extend(res.results.into_iter().map(Into::into));
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
    ) -> ProviderResult<Vec<Resource>> {
        use itertools::Itertools;

        let res = self
            .client
            .resources_to_spend(
                &from.hash().to_string(),
                vec![(format!("{asset_id:#x}").as_str(), amount, None)],
                None,
            )
            .await?
            .into_iter()
            .flatten()
            .map(|resource| {
                resource
                    .try_into()
                    .map_err(ProviderError::ClientRequestError)
            })
            .try_collect()?;

        Ok(res)
    }

    /// Get the balance of all spendable coins `asset_id` for address `address`. This is different
    /// from getting coins because we are just returning a number (the sum of UTXOs amount) instead
    /// of the UTXOs.
    pub async fn get_asset_balance(
        &self,
        address: &Bech32Address,
        asset_id: AssetId,
    ) -> ProviderResult<u64> {
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
    ) -> ProviderResult<u64> {
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
    ) -> ProviderResult<HashMap<String, u64>> {
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
    ) -> ProviderResult<HashMap<String, u64>> {
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

    pub async fn get_transaction_by_id(
        &self,
        tx_id: &str,
    ) -> ProviderResult<Option<TransactionResponse>> {
        Ok(self.client.transaction(tx_id).await?.map(Into::into))
    }

    // - Get transaction(s)
    pub async fn get_transactions(
        &self,
        request: PaginationRequest<String>,
    ) -> ProviderResult<PaginatedResult<TransactionResponse, String>> {
        let pr = self.client.transactions(request).await?;

        Ok(PaginatedResult {
            cursor: pr.cursor,
            results: pr.results.into_iter().map(Into::into).collect(),
            has_next_page: pr.has_next_page,
            has_previous_page: pr.has_previous_page,
        })
    }

    // Get transaction(s) by owner
    pub async fn get_transactions_by_owner(
        &self,
        owner: &Bech32Address,
        request: PaginationRequest<String>,
    ) -> ProviderResult<PaginatedResult<TransactionResponse, String>> {
        let pr = self
            .client
            .transactions_by_owner(&owner.hash().to_string(), request)
            .await?;

        Ok(PaginatedResult {
            cursor: pr.cursor,
            results: pr.results.into_iter().map(Into::into).collect(),
            has_next_page: pr.has_next_page,
            has_previous_page: pr.has_previous_page,
        })
    }

    pub async fn latest_block_height(&self) -> ProviderResult<u64> {
        Ok(self.client.chain_info().await?.latest_block.header.height.0)
    }

    pub async fn produce_blocks(
        &self,
        amount: u64,
        time: Option<TimeParameters>,
    ) -> io::Result<u64> {
        let fuel_time: Option<FuelTimeParameters> = time.map(|t| t.into());
        self.client.produce_blocks(amount, fuel_time).await
    }

    /// Get block by id.
    pub async fn block(&self, block_id: &str) -> ProviderResult<Option<Block>> {
        let block = self.client.block(block_id).await?.map(Into::into);
        Ok(block)
    }

    // - Get block(s)
    pub async fn get_blocks(
        &self,
        request: PaginationRequest<String>,
    ) -> ProviderResult<PaginatedResult<Block, String>> {
        let pr = self.client.blocks(request).await?;

        Ok(PaginatedResult {
            cursor: pr.cursor,
            results: pr.results.into_iter().map(Into::into).collect(),
            has_next_page: pr.has_next_page,
            has_previous_page: pr.has_previous_page,
        })
    }

    pub async fn estimate_transaction_cost<T: Transaction + Clone>(
        &self,
        tx: &T,
        tolerance: Option<f64>,
    ) -> Result<TransactionCost> {
        let NodeInfo { min_gas_price, .. } = self.node_info().await?;

        let tolerance = tolerance.unwrap_or(DEFAULT_GAS_ESTIMATION_TOLERANCE);
        let dry_run_tx = Self::generate_dry_run_tx(tx);
        let consensus_parameters = self.chain_info().await?.consensus_parameters;
        let gas_used = self
            .get_gas_used_with_tolerance(&dry_run_tx, tolerance)
            .await?;
        let gas_price = std::cmp::max(tx.gas_price(), min_gas_price);

        // Update the dry_run_tx with estimated gas_used and correct gas price to calculate the total_fee
        dry_run_tx
            .with_gas_price(gas_price)
            .with_gas_limit(gas_used);

        let transaction_fee = tx
            .fee_checked_from_tx(&consensus_parameters)
            .expect("Error calculating TransactionFee");

        Ok(TransactionCost {
            min_gas_price,
            gas_price,
            gas_used,
            metered_bytes_size: tx.metered_bytes_size() as u64,
            total_fee: transaction_fee.total(),
        })
    }

    // Remove limits from an existing Transaction to get an accurate gas estimation
    fn generate_dry_run_tx<T: Transaction + Clone>(tx: &T) -> T {
        // Simulate the contract call with MAX_GAS_PER_TX to get the complete gas_used
        tx.clone().with_gas_limit(MAX_GAS_PER_TX).with_gas_price(0)
    }

    // Increase estimated gas by the provided tolerance
    async fn get_gas_used_with_tolerance<T: Transaction + Clone>(
        &self,
        tx: &T,
        tolerance: f64,
    ) -> Result<u64> {
        let gas_used = self.get_gas_used(&self.dry_run_no_validation(tx).await?);
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

    pub async fn get_messages(&self, from: &Bech32Address) -> ProviderResult<Vec<Message>> {
        let pagination = PaginationRequest {
            cursor: None,
            results: 100,
            direction: PageDirection::Forward,
        };
        let res = self
            .client
            .messages(Some(&from.hash().to_string()), pagination)
            .await?
            .results
            .into_iter()
            .map(Into::into)
            .collect();
        Ok(res)
    }

    pub async fn get_message_proof(
        &self,
        tx_id: &str,
        message_id: &str,
    ) -> ProviderResult<Option<MessageProof>> {
        let proof = self
            .client
            .message_proof(tx_id, message_id)
            .await?
            .map(Into::into);
        Ok(proof)
    }
}
