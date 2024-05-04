use std::{collections::HashMap, fmt::Debug, net::SocketAddr};

mod retry_util;
mod retryable_client;
mod supported_versions;

#[cfg(feature = "coin-cache")]
use std::sync::Arc;

use chrono::{DateTime, Utc};
use fuel_core_client::client::{
    pagination::{PageDirection, PaginatedResult, PaginationRequest},
    types::{
        balance::Balance,
        contract::ContractBalance,
        gas_price::{EstimateGasPrice, LatestGasPrice},
    },
};
use fuel_core_types::services::executor::{TransactionExecutionResult, TransactionExecutionStatus};
use fuel_tx::{
    AssetId, ConsensusParameters, Receipt, Transaction as FuelTransaction, TxId, UtxoId,
};
use fuel_types::{Address, BlockHeight, Bytes32, ChainId, Nonce};
#[cfg(feature = "coin-cache")]
use fuels_core::types::coin_type_id::CoinTypeId;
use fuels_core::{
    constants::{DEFAULT_GAS_ESTIMATION_BLOCK_HORIZON, DEFAULT_GAS_ESTIMATION_TOLERANCE},
    types::{
        bech32::{Bech32Address, Bech32ContractId},
        block::Block,
        chain_info::ChainInfo,
        coin::Coin,
        coin_type::CoinType,
        errors::Result,
        message::Message,
        message_proof::MessageProof,
        node_info::NodeInfo,
        transaction::{Transaction, Transactions},
        transaction_builders::DryRunner,
        transaction_response::TransactionResponse,
        tx_status::TxStatus,
    },
};
pub use retry_util::{Backoff, RetryConfig};
use tai64::Tai64;
#[cfg(feature = "coin-cache")]
use tokio::sync::Mutex;

#[cfg(feature = "coin-cache")]
use crate::coin_cache::CoinsCache;
use crate::provider::retryable_client::RetryableClient;

#[derive(Debug)]
// ANCHOR: transaction_cost
pub struct TransactionCost {
    pub gas_price: u64,
    pub gas_used: u64,
    pub metered_bytes_size: u64,
    pub total_fee: u64,
}
// ANCHOR_END: transaction_cost

pub(crate) struct ResourceQueries {
    utxos: Vec<UtxoId>,
    messages: Vec<Nonce>,
    asset_id: Option<AssetId>,
    amount: u64,
}

impl ResourceQueries {
    pub fn exclusion_query(&self) -> Option<(Vec<UtxoId>, Vec<Nonce>)> {
        if self.utxos.is_empty() && self.messages.is_empty() {
            return None;
        }

        Some((self.utxos.clone(), self.messages.clone()))
    }

    pub fn spend_query(&self, base_asset_id: AssetId) -> Vec<(AssetId, u64, Option<u32>)> {
        vec![(self.asset_id.unwrap_or(base_asset_id), self.amount, None)]
    }
}

#[derive(Default)]
// ANCHOR: resource_filter
pub struct ResourceFilter {
    pub from: Bech32Address,
    pub asset_id: Option<AssetId>,
    pub amount: u64,
    pub excluded_utxos: Vec<UtxoId>,
    pub excluded_message_nonces: Vec<Nonce>,
}
// ANCHOR_END: resource_filter

impl ResourceFilter {
    pub fn owner(&self) -> Address {
        (&self.from).into()
    }

    pub(crate) fn resource_queries(&self) -> ResourceQueries {
        ResourceQueries {
            utxos: self.excluded_utxos.clone(),
            messages: self.excluded_message_nonces.clone(),
            asset_id: self.asset_id,
            amount: self.amount,
        }
    }
}

/// Encapsulates common client operations in the SDK.
/// Note that you may also use `client`, which is an instance
/// of `FuelClient`, directly, which provides a broader API.
#[derive(Debug, Clone)]
pub struct Provider {
    client: RetryableClient,
    consensus_parameters: ConsensusParameters,
    #[cfg(feature = "coin-cache")]
    cache: Arc<Mutex<CoinsCache>>,
}

impl Provider {
    pub async fn from(addr: impl Into<SocketAddr>) -> Result<Self> {
        let addr = addr.into();
        Self::connect(format!("http://{addr}")).await
    }

    pub async fn healthy(&self) -> Result<bool> {
        Ok(self.client.health().await?)
    }

    /// Connects to an existing node at the given address.
    pub async fn connect(url: impl AsRef<str>) -> Result<Provider> {
        let client = RetryableClient::connect(&url, Default::default()).await?;
        let consensus_parameters = client.chain_info().await?.consensus_parameters;

        Ok(Self {
            client,
            consensus_parameters,
            #[cfg(feature = "coin-cache")]
            cache: Default::default(),
        })
    }

    pub fn url(&self) -> &str {
        self.client.url()
    }

    /// Sends a transaction to the underlying Provider's client.
    pub async fn send_transaction_and_await_commit<T: Transaction>(
        &self,
        tx: T,
    ) -> Result<TxStatus> {
        let tx = self.prepare_transaction_for_sending(tx).await?;
        let tx_status = self
            .client
            .submit_and_await_commit(&tx.clone().into())
            .await?
            .into();

        #[cfg(feature = "coin-cache")]
        if matches!(
            tx_status,
            TxStatus::SqueezedOut { .. } | TxStatus::Revert { .. }
        ) {
            self.cache
                .lock()
                .await
                .remove_items(tx.used_coins(self.base_asset_id()))
        }

        Ok(tx_status)
    }

    async fn prepare_transaction_for_sending<T: Transaction>(&self, mut tx: T) -> Result<T> {
        tx.precompute(&self.chain_id())?;

        let chain_info = self.chain_info().await?;
        let latest_block_height = chain_info.latest_block.header.height;
        tx.check(latest_block_height, self.consensus_parameters())?;

        if tx.is_using_predicates() {
            tx.estimate_predicates(self.consensus_parameters())?;
            tx.clone()
                .validate_predicates(self.consensus_parameters(), latest_block_height)?;
        }

        self.validate_transaction(tx.clone()).await?;

        Ok(tx)
    }

    pub async fn send_transaction<T: Transaction>(&self, tx: T) -> Result<TxId> {
        let tx = self.prepare_transaction_for_sending(tx).await?;
        self.submit(tx).await
    }

    pub async fn await_transaction_commit<T: Transaction>(&self, id: TxId) -> Result<TxStatus> {
        Ok(self.client.await_transaction_commit(&id).await?.into())
    }

    async fn validate_transaction<T: Transaction>(&self, tx: T) -> Result<()> {
        let tolerance = 0.0;
        let TransactionCost { gas_used, .. } = self
            .estimate_transaction_cost(tx.clone(), Some(tolerance), None)
            .await?;

        tx.validate_gas(gas_used)?;

        Ok(())
    }

    #[cfg(not(feature = "coin-cache"))]
    async fn submit<T: Transaction>(&self, tx: T) -> Result<TxId> {
        Ok(self.client.submit(&tx.into()).await?)
    }

    #[cfg(feature = "coin-cache")]
    async fn submit<T: Transaction>(&self, tx: T) -> Result<TxId> {
        let used_utxos = tx.used_coins(self.base_asset_id());
        let tx_id = self.client.submit(&tx.into()).await?;
        self.cache.lock().await.insert_multiple(used_utxos);

        Ok(tx_id)
    }

    pub async fn tx_status(&self, tx_id: &TxId) -> Result<TxStatus> {
        Ok(self.client.transaction_status(tx_id).await?.into())
    }

    pub async fn chain_info(&self) -> Result<ChainInfo> {
        Ok(self.client.chain_info().await?.into())
    }

    pub fn consensus_parameters(&self) -> &ConsensusParameters {
        &self.consensus_parameters
    }

    pub fn base_asset_id(&self) -> &AssetId {
        self.consensus_parameters.base_asset_id()
    }

    pub fn chain_id(&self) -> ChainId {
        self.consensus_parameters.chain_id()
    }

    pub async fn node_info(&self) -> Result<NodeInfo> {
        Ok(self.client.node_info().await?.into())
    }

    pub async fn latest_gas_price(&self) -> Result<LatestGasPrice> {
        Ok(self.client.latest_gas_price().await?)
    }

    pub async fn estimate_gas_price(&self, block_horizon: u32) -> Result<EstimateGasPrice> {
        Ok(self.client.estimate_gas_price(block_horizon).await?)
    }

    pub async fn dry_run(&self, tx: impl Transaction) -> Result<TxStatus> {
        let [(_, tx_status)] = self
            .client
            .dry_run(Transactions::new().insert(tx).as_slice())
            .await?
            .into_iter()
            .map(Self::tx_status_from_execution_status)
            .collect::<Vec<_>>()
            .try_into()
            .expect("should have only one element");

        Ok(tx_status)
    }

    pub async fn dry_run_multiple(
        &self,
        transactions: Transactions,
    ) -> Result<Vec<(TxId, TxStatus)>> {
        Ok(self
            .client
            .dry_run(transactions.as_slice())
            .await?
            .into_iter()
            .map(Self::tx_status_from_execution_status)
            .collect())
    }

    fn tx_status_from_execution_status(
        tx_execution_status: TransactionExecutionStatus,
    ) -> (TxId, TxStatus) {
        (
            tx_execution_status.id,
            match tx_execution_status.result {
                TransactionExecutionResult::Success { receipts, .. } => {
                    TxStatus::Success { receipts }
                }
                TransactionExecutionResult::Failed {
                    receipts, result, ..
                } => TxStatus::Revert {
                    reason: TransactionExecutionResult::reason(&receipts, &result),
                    receipts,
                    revert_id: 0,
                },
            },
        )
    }

    pub async fn dry_run_no_validation(&self, tx: impl Transaction) -> Result<TxStatus> {
        let [(_, tx_status)] = self
            .client
            .dry_run_opt(Transactions::new().insert(tx).as_slice(), Some(false))
            .await?
            .into_iter()
            .map(Self::tx_status_from_execution_status)
            .collect::<Vec<_>>()
            .try_into()
            .expect("should have only one element");

        Ok(tx_status)
    }

    pub async fn dry_run_no_validation_multiple(
        &self,
        transactions: Transactions,
    ) -> Result<Vec<(TxId, TxStatus)>> {
        Ok(self
            .client
            .dry_run_opt(transactions.as_slice(), Some(false))
            .await?
            .into_iter()
            .map(Self::tx_status_from_execution_status)
            .collect())
    }

    /// Gets all unspent coins owned by address `from`, with asset ID `asset_id`.
    pub async fn get_coins(&self, from: &Bech32Address, asset_id: AssetId) -> Result<Vec<Coin>> {
        let mut coins: Vec<Coin> = vec![];

        let mut cursor = None;

        loop {
            let res = self
                .client
                .coins(
                    &from.into(),
                    Some(&asset_id),
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

    async fn request_coins_to_spend(&self, filter: ResourceFilter) -> Result<Vec<CoinType>> {
        let queries = filter.resource_queries();

        let res = self
            .client
            .coins_to_spend(
                &filter.owner(),
                queries.spend_query(*self.base_asset_id()),
                queries.exclusion_query(),
            )
            .await?
            .into_iter()
            .flatten()
            .map(CoinType::try_from)
            .collect::<Result<Vec<CoinType>>>()?;

        Ok(res)
    }

    /// Get some spendable coins of asset `asset_id` for address `from` that add up at least to
    /// amount `amount`. The returned coins (UTXOs) are actual coins that can be spent. The number
    /// of coins (UXTOs) is optimized to prevent dust accumulation.
    #[cfg(not(feature = "coin-cache"))]
    pub async fn get_spendable_resources(&self, filter: ResourceFilter) -> Result<Vec<CoinType>> {
        self.request_coins_to_spend(filter).await
    }

    /// Get some spendable coins of asset `asset_id` for address `from` that add up at least to
    /// amount `amount`. The returned coins (UTXOs) are actual coins that can be spent. The number
    /// of coins (UXTOs) is optimized to prevent dust accumulation.
    /// Coins that were recently submitted inside a tx will be ignored from the results.
    #[cfg(feature = "coin-cache")]
    pub async fn get_spendable_resources(
        &self,
        mut filter: ResourceFilter,
    ) -> Result<Vec<CoinType>> {
        self.extend_filter_with_cached(&mut filter).await;

        self.request_coins_to_spend(filter).await
    }

    #[cfg(feature = "coin-cache")]
    async fn extend_filter_with_cached(&self, filter: &mut ResourceFilter) {
        let mut cache = self.cache.lock().await;
        let asset_id = filter.asset_id.unwrap_or(*self.base_asset_id());
        let used_coins = cache.get_active(&(filter.from.clone(), asset_id));

        let excluded_utxos = used_coins
            .iter()
            .filter_map(|coin_id| match coin_id {
                CoinTypeId::UtxoId(utxo_id) => Some(utxo_id),
                _ => None,
            })
            .cloned()
            .collect::<Vec<_>>();

        let excluded_message_nonces = used_coins
            .iter()
            .filter_map(|coin_id| match coin_id {
                CoinTypeId::Nonce(nonce) => Some(nonce),
                _ => None,
            })
            .cloned()
            .collect::<Vec<_>>();

        filter.excluded_utxos.extend(excluded_utxos);
        filter
            .excluded_message_nonces
            .extend(excluded_message_nonces);
    }

    /// Get the balance of all spendable coins `asset_id` for address `address`. This is different
    /// from getting coins because we are just returning a number (the sum of UTXOs amount) instead
    /// of the UTXOs.
    pub async fn get_asset_balance(
        &self,
        address: &Bech32Address,
        asset_id: AssetId,
    ) -> Result<u64> {
        Ok(self
            .client
            .balance(&address.into(), Some(&asset_id))
            .await?)
    }

    /// Get the balance of all spendable coins `asset_id` for contract with id `contract_id`.
    pub async fn get_contract_asset_balance(
        &self,
        contract_id: &Bech32ContractId,
        asset_id: AssetId,
    ) -> Result<u64> {
        Ok(self
            .client
            .contract_balance(&contract_id.into(), Some(&asset_id))
            .await?)
    }

    /// Get all the spendable balances of all assets for address `address`. This is different from
    /// getting the coins because we are only returning the numbers (the sum of UTXOs coins amount
    /// for each asset id) and not the UTXOs coins themselves
    pub async fn get_balances(&self, address: &Bech32Address) -> Result<HashMap<String, u64>> {
        // We don't paginate results because there are likely at most ~100 different assets in one
        // wallet
        let pagination = PaginationRequest {
            cursor: None,
            results: 9999,
            direction: PageDirection::Forward,
        };
        let balances_vec = self
            .client
            .balances(&address.into(), pagination)
            .await?
            .results;
        let balances = balances_vec
            .into_iter()
            .map(
                |Balance {
                     owner: _,
                     amount,
                     asset_id,
                 }| (asset_id.to_string(), amount),
            )
            .collect();
        Ok(balances)
    }

    /// Get all balances of all assets for the contract with id `contract_id`.
    pub async fn get_contract_balances(
        &self,
        contract_id: &Bech32ContractId,
    ) -> Result<HashMap<AssetId, u64>> {
        // We don't paginate results because there are likely at most ~100 different assets in one
        // wallet
        let pagination = PaginationRequest {
            cursor: None,
            results: 9999,
            direction: PageDirection::Forward,
        };

        let balances_vec = self
            .client
            .contract_balances(&contract_id.into(), pagination)
            .await?
            .results;
        let balances = balances_vec
            .into_iter()
            .map(
                |ContractBalance {
                     contract: _,
                     amount,
                     asset_id,
                 }| (asset_id, amount),
            )
            .collect();
        Ok(balances)
    }

    pub async fn get_transaction_by_id(&self, tx_id: &TxId) -> Result<Option<TransactionResponse>> {
        Ok(self.client.transaction(tx_id).await?.map(Into::into))
    }

    pub async fn get_transactions(
        &self,
        request: PaginationRequest<String>,
    ) -> Result<PaginatedResult<TransactionResponse, String>> {
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
    ) -> Result<PaginatedResult<TransactionResponse, String>> {
        let pr = self
            .client
            .transactions_by_owner(&owner.into(), request)
            .await?;

        Ok(PaginatedResult {
            cursor: pr.cursor,
            results: pr.results.into_iter().map(Into::into).collect(),
            has_next_page: pr.has_next_page,
            has_previous_page: pr.has_previous_page,
        })
    }

    pub async fn latest_block_height(&self) -> Result<u32> {
        Ok(self.chain_info().await?.latest_block.header.height)
    }

    pub async fn latest_block_time(&self) -> Result<Option<DateTime<Utc>>> {
        Ok(self.chain_info().await?.latest_block.header.time)
    }

    pub async fn produce_blocks(
        &self,
        blocks_to_produce: u32,
        start_time: Option<DateTime<Utc>>,
    ) -> Result<u32> {
        let start_time = start_time.map(|time| Tai64::from_unix(time.timestamp()).0);

        Ok(self
            .client
            .produce_blocks(blocks_to_produce, start_time)
            .await?
            .into())
    }

    pub async fn block(&self, block_id: &Bytes32) -> Result<Option<Block>> {
        Ok(self.client.block(block_id).await?.map(Into::into))
    }

    pub async fn block_by_height(&self, height: BlockHeight) -> Result<Option<Block>> {
        Ok(self.client.block_by_height(height).await?.map(Into::into))
    }

    // - Get block(s)
    pub async fn get_blocks(
        &self,
        request: PaginationRequest<String>,
    ) -> Result<PaginatedResult<Block, String>> {
        let pr = self.client.blocks(request).await?;

        Ok(PaginatedResult {
            cursor: pr.cursor,
            results: pr.results.into_iter().map(Into::into).collect(),
            has_next_page: pr.has_next_page,
            has_previous_page: pr.has_previous_page,
        })
    }

    pub async fn estimate_transaction_cost<T: Transaction>(
        &self,
        tx: T,
        tolerance: Option<f64>,
        block_horizon: Option<u32>,
    ) -> Result<TransactionCost> {
        let block_horizon = block_horizon.unwrap_or(DEFAULT_GAS_ESTIMATION_BLOCK_HORIZON);
        let tolerance = tolerance.unwrap_or(DEFAULT_GAS_ESTIMATION_TOLERANCE);

        let EstimateGasPrice { gas_price, .. } = self.estimate_gas_price(block_horizon).await?;

        let gas_used = self
            .get_gas_used_with_tolerance(tx.clone(), tolerance)
            .await?;

        let transaction_fee = tx
            .clone()
            .fee_checked_from_tx(&self.consensus_parameters, gas_price)
            .expect("Error calculating TransactionFee");

        Ok(TransactionCost {
            gas_price,
            gas_used,
            metered_bytes_size: tx.metered_bytes_size() as u64,
            total_fee: transaction_fee.max_fee(),
        })
    }

    // Increase estimated gas by the provided tolerance
    async fn get_gas_used_with_tolerance<T: Transaction>(
        &self,
        tx: T,
        tolerance: f64,
    ) -> Result<u64> {
        let receipts = self.dry_run_no_validation(tx).await?.take_receipts();
        let gas_used = self.get_gas_used(&receipts);

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

    pub async fn get_messages(&self, from: &Bech32Address) -> Result<Vec<Message>> {
        let pagination = PaginationRequest {
            cursor: None,
            results: 100,
            direction: PageDirection::Forward,
        };

        Ok(self
            .client
            .messages(Some(&from.into()), pagination)
            .await?
            .results
            .into_iter()
            .map(Into::into)
            .collect())
    }

    pub async fn get_message_proof(
        &self,
        tx_id: &TxId,
        nonce: &Nonce,
        commit_block_id: Option<&Bytes32>,
        commit_block_height: Option<u32>,
    ) -> Result<Option<MessageProof>> {
        let proof = self
            .client
            .message_proof(
                tx_id,
                nonce,
                commit_block_id.map(Into::into),
                commit_block_height.map(Into::into),
            )
            .await?
            .map(Into::into);

        Ok(proof)
    }

    pub fn with_retry_config(mut self, retry_config: RetryConfig) -> Self {
        self.client.set_retry_config(retry_config);

        self
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl DryRunner for Provider {
    async fn dry_run_and_get_used_gas(&self, tx: FuelTransaction, tolerance: f32) -> Result<u64> {
        let [tx_execution_status] = self
            .client
            .dry_run_opt(&vec![tx], Some(false))
            .await?
            .try_into()
            .expect("should have only one element");

        let gas_used = self.get_gas_used(tx_execution_status.result.receipts());

        Ok((gas_used as f64 * (1.0 + tolerance as f64)) as u64)
    }

    async fn estimate_gas_price(&self, block_horizon: u32) -> Result<u64> {
        Ok(self.estimate_gas_price(block_horizon).await?.gas_price)
    }

    fn consensus_parameters(&self) -> &ConsensusParameters {
        self.consensus_parameters()
    }
}
