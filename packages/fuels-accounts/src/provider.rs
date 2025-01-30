#[cfg(feature = "coin-cache")]
use std::sync::Arc;
use std::{collections::HashMap, fmt::Debug, net::SocketAddr};

mod cache;
mod retry_util;
mod retryable_client;
mod supported_fuel_core_version;
mod supported_versions;

use crate::provider::cache::CacheableRpcs;
pub use cache::TtlConfig;
use cache::{CachedClient, SystemClock};
use chrono::{DateTime, Utc};
use fuel_core_client::client::{
    pagination::{PageDirection, PaginatedResult, PaginationRequest},
    types::{
        balance::Balance,
        contract::ContractBalance,
        gas_price::{EstimateGasPrice, LatestGasPrice},
    },
};
use fuel_core_types::services::executor::TransactionExecutionResult;
use fuel_tx::{
    AssetId, ConsensusParameters, Receipt, Transaction as FuelTransaction, TxId, UtxoId,
};
use fuel_types::{Address, BlockHeight, Bytes32, Nonce};
#[cfg(feature = "coin-cache")]
use fuels_core::types::coin_type_id::CoinTypeId;
use fuels_core::{
    constants::{DEFAULT_GAS_ESTIMATION_BLOCK_HORIZON, DEFAULT_GAS_ESTIMATION_TOLERANCE},
    types::{
        bech32::{Bech32Address, Bech32ContractId},
        block::{Block, Header},
        chain_info::ChainInfo,
        coin::Coin,
        coin_type::CoinType,
        errors::Result,
        message::Message,
        message_proof::MessageProof,
        node_info::NodeInfo,
        transaction::{Transaction, Transactions},
        transaction_builders::{Blob, BlobId},
        transaction_response::TransactionResponse,
        tx_status::TxStatus,
        DryRun, DryRunner,
    },
};
pub use retry_util::{Backoff, RetryConfig};
pub use supported_fuel_core_version::SUPPORTED_FUEL_CORE_VERSION;
use tai64::Tai64;
#[cfg(feature = "coin-cache")]
use tokio::sync::Mutex;

#[cfg(feature = "coin-cache")]
use crate::coin_cache::CoinsCache;
use crate::provider::retryable_client::RetryableClient;

const NUM_RESULTS_PER_REQUEST: i32 = 100;

#[derive(Debug, Clone, PartialEq)]
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
    cached_client: CachedClient<RetryableClient>,
    #[cfg(feature = "coin-cache")]
    coins_cache: Arc<Mutex<CoinsCache>>,
}

impl Provider {
    pub async fn from(addr: impl Into<SocketAddr>) -> Result<Self> {
        let addr = addr.into();
        Self::connect(format!("http://{addr}")).await
    }

    pub fn set_cache_ttl(&mut self, ttl: TtlConfig) {
        self.cached_client.set_ttl(ttl);
    }

    pub async fn clear_cache(&self) {
        self.cached_client.clear().await;
    }

    pub async fn healthy(&self) -> Result<bool> {
        Ok(self.uncached_client().health().await?)
    }

    /// Connects to an existing node at the given address.
    pub async fn connect(url: impl AsRef<str>) -> Result<Provider> {
        let client = CachedClient::new(
            RetryableClient::connect(&url, Default::default()).await?,
            TtlConfig::default(),
            SystemClock,
        );

        Ok(Self {
            cached_client: client,
            #[cfg(feature = "coin-cache")]
            coins_cache: Default::default(),
        })
    }

    pub fn url(&self) -> &str {
        self.uncached_client().url()
    }

    pub async fn blob(&self, blob_id: BlobId) -> Result<Option<Blob>> {
        Ok(self
            .uncached_client()
            .blob(blob_id.into())
            .await?
            .map(|blob| Blob::new(blob.bytecode)))
    }

    pub async fn blob_exists(&self, blob_id: BlobId) -> Result<bool> {
        Ok(self.uncached_client().blob_exists(blob_id.into()).await?)
    }

    /// Sends a transaction to the underlying Provider's client.
    pub async fn send_transaction_and_await_commit<T: Transaction>(
        &self,
        tx: T,
    ) -> Result<TxStatus> {
        #[cfg(feature = "coin-cache")]
        let base_asset_id = *self.consensus_parameters().await?.base_asset_id();

        #[cfg(feature = "coin-cache")]
        self.check_inputs_already_in_cache(&tx.used_coins(&base_asset_id))
            .await?;

        let tx = self.prepare_transaction_for_sending(tx).await?;
        let tx_status = self
            .uncached_client()
            .submit_and_await_commit(&tx.clone().into())
            .await?
            .into();

        #[cfg(feature = "coin-cache")]
        if matches!(
            tx_status,
            TxStatus::SqueezedOut { .. } | TxStatus::Revert { .. }
        ) {
            self.coins_cache
                .lock()
                .await
                .remove_items(tx.used_coins(&base_asset_id))
        }

        Ok(tx_status)
    }

    async fn prepare_transaction_for_sending<T: Transaction>(&self, mut tx: T) -> Result<T> {
        let consensus_parameters = self.consensus_parameters().await?;
        tx.precompute(&consensus_parameters.chain_id())?;

        let chain_info = self.chain_info().await?;
        let Header {
            height: latest_block_height,
            state_transition_bytecode_version: latest_chain_executor_version,
            ..
        } = chain_info.latest_block.header;

        if tx.is_using_predicates() {
            tx.estimate_predicates(self, Some(latest_chain_executor_version))
                .await?;
            tx.clone()
                .validate_predicates(&consensus_parameters, latest_block_height)?;
        }

        self.validate_transaction(tx.clone()).await?;

        Ok(tx)
    }

    pub async fn send_transaction<T: Transaction>(&self, tx: T) -> Result<TxId> {
        let tx = self.prepare_transaction_for_sending(tx).await?;
        self.submit(tx).await
    }

    pub async fn await_transaction_commit<T: Transaction>(&self, id: TxId) -> Result<TxStatus> {
        Ok(self
            .uncached_client()
            .await_transaction_commit(&id)
            .await?
            .into())
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
        Ok(self.uncached_client().submit(&tx.into()).await?)
    }

    #[cfg(feature = "coin-cache")]
    async fn find_in_cache<'a>(
        &self,
        coin_ids: impl IntoIterator<Item = (&'a (Bech32Address, AssetId), &'a Vec<CoinTypeId>)>,
    ) -> Option<((Bech32Address, AssetId), CoinTypeId)> {
        let mut locked_cache = self.coins_cache.lock().await;

        for (key, ids) in coin_ids {
            let items = locked_cache.get_active(key);

            if items.is_empty() {
                continue;
            }

            for id in ids {
                if items.contains(id) {
                    return Some((key.clone(), id.clone()));
                }
            }
        }

        None
    }

    #[cfg(feature = "coin-cache")]
    async fn check_inputs_already_in_cache<'a>(
        &self,
        coin_ids: impl IntoIterator<Item = (&'a (Bech32Address, AssetId), &'a Vec<CoinTypeId>)>,
    ) -> Result<()> {
        use fuels_core::types::errors::{transaction, Error};

        if let Some(((addr, asset_id), coin_type_id)) = self.find_in_cache(coin_ids).await {
            let msg = match coin_type_id {
                CoinTypeId::UtxoId(utxo_id) => format!("coin with utxo_id: `{utxo_id:x}`"),
                CoinTypeId::Nonce(nonce) => format!("message with nonce: `{nonce}`"),
            };
            Err(Error::Transaction(transaction::Reason::Validation(
                format!("{msg} was submitted recently in a transaction - attempting to spend it again will result in an error. Wallet address: `{addr}`, asset id: `{asset_id}`"),
            )))
        } else {
            Ok(())
        }
    }

    #[cfg(feature = "coin-cache")]
    async fn submit<T: Transaction>(&self, tx: T) -> Result<TxId> {
        let consensus_parameters = self.consensus_parameters().await?;
        let base_asset_id = consensus_parameters.base_asset_id();

        let used_utxos = tx.used_coins(base_asset_id);
        self.check_inputs_already_in_cache(&used_utxos).await?;

        let tx_id = self.uncached_client().submit(&tx.into()).await?;
        self.coins_cache.lock().await.insert_multiple(used_utxos);

        Ok(tx_id)
    }

    pub async fn tx_status(&self, tx_id: &TxId) -> Result<TxStatus> {
        Ok(self
            .uncached_client()
            .transaction_status(tx_id)
            .await?
            .into())
    }

    pub async fn chain_info(&self) -> Result<ChainInfo> {
        Ok(self.uncached_client().chain_info().await?.into())
    }

    pub async fn consensus_parameters(&self) -> Result<ConsensusParameters> {
        self.cached_client.consensus_parameters().await
    }

    pub async fn node_info(&self) -> Result<NodeInfo> {
        Ok(self.uncached_client().node_info().await?.into())
    }

    pub async fn latest_gas_price(&self) -> Result<LatestGasPrice> {
        Ok(self.uncached_client().latest_gas_price().await?)
    }

    pub async fn estimate_gas_price(&self, block_horizon: u32) -> Result<EstimateGasPrice> {
        Ok(self
            .uncached_client()
            .estimate_gas_price(block_horizon)
            .await?)
    }

    pub async fn dry_run(&self, tx: impl Transaction) -> Result<TxStatus> {
        let [tx_status] = self
            .uncached_client()
            .dry_run(Transactions::new().insert(tx).as_slice())
            .await?
            .into_iter()
            .map(Into::into)
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
            .uncached_client()
            .dry_run(transactions.as_slice())
            .await?
            .into_iter()
            .map(|execution_status| (execution_status.id, execution_status.into()))
            .collect())
    }

    pub async fn dry_run_opt(
        &self,
        tx: impl Transaction,
        utxo_validation: bool,
        gas_price: Option<u64>,
    ) -> Result<TxStatus> {
        let [tx_status] = self
            .uncached_client()
            .dry_run_opt(
                Transactions::new().insert(tx).as_slice(),
                Some(utxo_validation),
                gas_price,
            )
            .await?
            .into_iter()
            .map(Into::into)
            .collect::<Vec<_>>()
            .try_into()
            .expect("should have only one element");

        Ok(tx_status)
    }

    pub async fn dry_run_opt_multiple(
        &self,
        transactions: Transactions,
        utxo_validation: bool,
        gas_price: Option<u64>,
    ) -> Result<Vec<(TxId, TxStatus)>> {
        Ok(self
            .uncached_client()
            .dry_run_opt(transactions.as_slice(), Some(utxo_validation), gas_price)
            .await?
            .into_iter()
            .map(|execution_status| (execution_status.id, execution_status.into()))
            .collect())
    }

    /// Gets all unspent coins owned by address `from`, with asset ID `asset_id`.
    pub async fn get_coins(&self, from: &Bech32Address, asset_id: AssetId) -> Result<Vec<Coin>> {
        let mut coins: Vec<Coin> = vec![];
        let mut cursor = None;

        loop {
            let response = self
                .uncached_client()
                .coins(
                    &from.into(),
                    Some(&asset_id),
                    PaginationRequest {
                        cursor: cursor.clone(),
                        results: NUM_RESULTS_PER_REQUEST,
                        direction: PageDirection::Forward,
                    },
                )
                .await?;

            if response.results.is_empty() {
                break;
            }

            coins.extend(response.results.into_iter().map(Into::into));
            cursor = response.cursor;
        }

        Ok(coins)
    }

    async fn request_coins_to_spend(&self, filter: ResourceFilter) -> Result<Vec<CoinType>> {
        let queries = filter.resource_queries();

        let consensus_parameters = self.consensus_parameters().await?;
        let base_asset_id = *consensus_parameters.base_asset_id();

        let res = self
            .uncached_client()
            .coins_to_spend(
                &filter.owner(),
                queries.spend_query(base_asset_id),
                queries.exclusion_query(),
            )
            .await?
            .into_iter()
            .flatten()
            .map(CoinType::from)
            .collect();

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
        self.extend_filter_with_cached(&mut filter).await?;

        self.request_coins_to_spend(filter).await
    }

    #[cfg(feature = "coin-cache")]
    async fn extend_filter_with_cached(&self, filter: &mut ResourceFilter) -> Result<()> {
        let consensus_parameters = self.consensus_parameters().await?;
        let mut cache = self.coins_cache.lock().await;
        let asset_id = filter
            .asset_id
            .unwrap_or(*consensus_parameters.base_asset_id());
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

        Ok(())
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
            .uncached_client()
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
            .uncached_client()
            .contract_balance(&contract_id.into(), Some(&asset_id))
            .await?)
    }

    /// Get all the spendable balances of all assets for address `address`. This is different from
    /// getting the coins because we are only returning the numbers (the sum of UTXOs coins amount
    /// for each asset id) and not the UTXOs coins themselves
    pub async fn get_balances(&self, address: &Bech32Address) -> Result<HashMap<String, u128>> {
        let mut balances = HashMap::new();
        let mut cursor = None;

        loop {
            let response = self
                .uncached_client()
                .balances(
                    &address.into(),
                    PaginationRequest {
                        cursor: cursor.clone(),
                        results: NUM_RESULTS_PER_REQUEST,
                        direction: PageDirection::Forward,
                    },
                )
                .await?;

            if response.results.is_empty() {
                break;
            }

            balances.extend(response.results.into_iter().map(
                |Balance {
                     owner: _,
                     amount,
                     asset_id,
                 }| (asset_id.to_string(), amount),
            ));
            cursor = response.cursor;
        }

        Ok(balances)
    }

    /// Get all balances of all assets for the contract with id `contract_id`.
    pub async fn get_contract_balances(
        &self,
        contract_id: &Bech32ContractId,
    ) -> Result<HashMap<AssetId, u64>> {
        let mut contract_balances = HashMap::new();
        let mut cursor = None;

        loop {
            let response = self
                .uncached_client()
                .contract_balances(
                    &contract_id.into(),
                    PaginationRequest {
                        cursor: cursor.clone(),
                        results: NUM_RESULTS_PER_REQUEST,
                        direction: PageDirection::Forward,
                    },
                )
                .await?;

            if response.results.is_empty() {
                break;
            }

            contract_balances.extend(response.results.into_iter().map(
                |ContractBalance {
                     contract: _,
                     amount,
                     asset_id,
                 }| (asset_id, amount),
            ));
            cursor = response.cursor;
        }

        Ok(contract_balances)
    }

    pub async fn get_transaction_by_id(&self, tx_id: &TxId) -> Result<Option<TransactionResponse>> {
        Ok(self
            .uncached_client()
            .transaction(tx_id)
            .await?
            .map(Into::into))
    }

    pub async fn get_transactions(
        &self,
        request: PaginationRequest<String>,
    ) -> Result<PaginatedResult<TransactionResponse, String>> {
        let pr = self.uncached_client().transactions(request).await?;

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
            .uncached_client()
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
            .uncached_client()
            .produce_blocks(blocks_to_produce, start_time)
            .await?
            .into())
    }

    pub async fn block(&self, block_id: &Bytes32) -> Result<Option<Block>> {
        Ok(self
            .uncached_client()
            .block(block_id)
            .await?
            .map(Into::into))
    }

    pub async fn block_by_height(&self, height: BlockHeight) -> Result<Option<Block>> {
        Ok(self
            .uncached_client()
            .block_by_height(height)
            .await?
            .map(Into::into))
    }

    // - Get block(s)
    pub async fn get_blocks(
        &self,
        request: PaginationRequest<String>,
    ) -> Result<PaginatedResult<Block, String>> {
        let pr = self.uncached_client().blocks(request).await?;

        Ok(PaginatedResult {
            cursor: pr.cursor,
            results: pr.results.into_iter().map(Into::into).collect(),
            has_next_page: pr.has_next_page,
            has_previous_page: pr.has_previous_page,
        })
    }

    pub async fn estimate_transaction_cost<T: Transaction>(
        &self,
        mut tx: T,
        tolerance: Option<f64>,
        block_horizon: Option<u32>,
    ) -> Result<TransactionCost> {
        let block_horizon = block_horizon.unwrap_or(DEFAULT_GAS_ESTIMATION_BLOCK_HORIZON);
        let tolerance = tolerance.unwrap_or(DEFAULT_GAS_ESTIMATION_TOLERANCE);

        let EstimateGasPrice { gas_price, .. } = self.estimate_gas_price(block_horizon).await?;

        let gas_used = self
            .get_gas_used_with_tolerance(tx.clone(), tolerance)
            .await?;

        if tx.is_using_predicates() {
            tx.estimate_predicates(self, None).await?;
        }

        let transaction_fee = tx
            .clone()
            .fee_checked_from_tx(&self.consensus_parameters().await?, gas_price)
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
        let receipts = self.dry_run_opt(tx, false, None).await?.take_receipts();
        let gas_used = self.get_script_gas_used(&receipts);

        Ok((gas_used as f64 * (1.0 + tolerance)).ceil() as u64)
    }

    fn get_script_gas_used(&self, receipts: &[Receipt]) -> u64 {
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
        let mut messages = Vec::new();
        let mut cursor = None;

        loop {
            let response = self
                .uncached_client()
                .messages(
                    Some(&from.into()),
                    PaginationRequest {
                        cursor: cursor.clone(),
                        results: NUM_RESULTS_PER_REQUEST,
                        direction: PageDirection::Forward,
                    },
                )
                .await?;

            if response.results.is_empty() {
                break;
            }

            messages.extend(response.results.into_iter().map(Into::into));
            cursor = response.cursor;
        }

        Ok(messages)
    }

    pub async fn get_message_proof(
        &self,
        tx_id: &TxId,
        nonce: &Nonce,
        commit_block_id: Option<&Bytes32>,
        commit_block_height: Option<u32>,
    ) -> Result<MessageProof> {
        self.uncached_client()
            .message_proof(
                tx_id,
                nonce,
                commit_block_id.map(Into::into),
                commit_block_height.map(Into::into),
            )
            .await
            .map(Into::into)
            .map_err(Into::into)
    }

    pub async fn is_user_account(&self, address: impl Into<Bytes32>) -> Result<bool> {
        self.uncached_client()
            .is_user_account(*address.into())
            .await
    }

    pub fn with_retry_config(mut self, retry_config: RetryConfig) -> Self {
        self.uncached_client_mut().set_retry_config(retry_config);

        self
    }

    pub async fn contract_exists(&self, contract_id: &Bech32ContractId) -> Result<bool> {
        Ok(self
            .uncached_client()
            .contract_exists(&contract_id.into())
            .await?)
    }

    fn uncached_client(&self) -> &RetryableClient {
        self.cached_client.inner()
    }

    fn uncached_client_mut(&mut self) -> &mut RetryableClient {
        self.cached_client.inner_mut()
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl DryRunner for Provider {
    async fn dry_run(&self, tx: FuelTransaction) -> Result<DryRun> {
        let [tx_execution_status] = self
            .uncached_client()
            .dry_run_opt(&vec![tx], Some(false), Some(0))
            .await?
            .try_into()
            .expect("should have only one element");

        let receipts = tx_execution_status.result.receipts();
        let script_gas = self.get_script_gas_used(receipts);

        let variable_outputs = receipts
            .iter()
            .filter(
                |receipt| matches!(receipt, Receipt::TransferOut { amount, .. } if *amount != 0),
            )
            .count();

        let succeeded = matches!(
            tx_execution_status.result,
            TransactionExecutionResult::Success { .. }
        );

        let dry_run = DryRun {
            succeeded,
            script_gas,
            variable_outputs,
        };

        Ok(dry_run)
    }

    async fn estimate_gas_price(&self, block_horizon: u32) -> Result<u64> {
        Ok(self.estimate_gas_price(block_horizon).await?.gas_price)
    }

    async fn estimate_predicates(
        &self,
        tx: &FuelTransaction,
        _latest_chain_executor_version: Option<u32>,
    ) -> Result<FuelTransaction> {
        Ok(self.uncached_client().estimate_predicates(tx).await?)
    }

    async fn consensus_parameters(&self) -> Result<ConsensusParameters> {
        Provider::consensus_parameters(self).await
    }
}
