use std::{collections::HashMap, fmt::Debug, io, net::SocketAddr};

mod retry_util;
mod retryable_client;
mod supported_versions;

#[cfg(feature = "coin-cache")]
use std::sync::Arc;

use chrono::{DateTime, Utc};
#[cfg(feature = "coin-cache")]
use fuel_core_client::client::types::TransactionStatus;
use fuel_core_client::client::{
    pagination::{PageDirection, PaginatedResult, PaginationRequest},
    types::{balance::Balance, contract::ContractBalance},
};
use fuel_tx::{
    AssetId, ConsensusParameters, Receipt, ScriptExecutionResult, Transaction as FuelTransaction,
    TxId, UtxoId,
};
use fuel_types::{Address, Bytes32, ChainId, Nonce};
#[cfg(feature = "coin-cache")]
use fuels_core::types::coin_type_id::CoinTypeId;
use fuels_core::{
    constants::{BASE_ASSET_ID, DEFAULT_GAS_ESTIMATION_TOLERANCE},
    types::{
        bech32::{Bech32Address, Bech32ContractId},
        block::Block,
        chain_info::ChainInfo,
        coin::Coin,
        coin_type::CoinType,
        errors::{error, Error, Result},
        message::Message,
        message_proof::MessageProof,
        node_info::NodeInfo,
        transaction::Transaction,
        transaction_builders::DryRunner,
        transaction_response::TransactionResponse,
        tx_status::TxStatus,
    },
};
pub use retry_util::{Backoff, RetryConfig};
use supported_versions::{check_fuel_core_version_compatibility, VersionCompatibility};
use tai64::Tai64;
use thiserror::Error;
#[cfg(feature = "coin-cache")]
use tokio::sync::Mutex;

#[cfg(feature = "coin-cache")]
use crate::coin_cache::CoinsCache;
use crate::provider::retryable_client::RetryableClient;

type ProviderResult<T> = std::result::Result<T, ProviderError>;

#[derive(Debug)]
pub struct TransactionCost {
    pub min_gas_price: u64,
    pub gas_price: u64,
    pub gas_used: u64,
    pub metered_bytes_size: u64,
    pub total_fee: u64,
}

pub(crate) struct ResourceQueries {
    utxos: Vec<UtxoId>,
    messages: Vec<Nonce>,
    asset_id: AssetId,
    amount: u64,
}

impl ResourceQueries {
    pub fn new(
        utxo_ids: Vec<UtxoId>,
        message_nonces: Vec<Nonce>,
        asset_id: AssetId,
        amount: u64,
    ) -> Self {
        Self {
            utxos: utxo_ids,
            messages: message_nonces,
            asset_id,
            amount,
        }
    }

    pub fn exclusion_query(&self) -> Option<(Vec<UtxoId>, Vec<Nonce>)> {
        if self.utxos.is_empty() && self.messages.is_empty() {
            return None;
        }

        Some((self.utxos.clone(), self.messages.clone()))
    }

    pub fn spend_query(&self) -> Vec<(AssetId, u64, Option<u32>)> {
        vec![(self.asset_id, self.amount, None)]
    }
}

// ANCHOR: resource_filter
pub struct ResourceFilter {
    pub from: Bech32Address,
    pub asset_id: AssetId,
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
        ResourceQueries::new(
            self.excluded_utxos.clone(),
            self.excluded_message_nonces.clone(),
            self.asset_id,
            self.amount,
        )
    }
}

impl Default for ResourceFilter {
    fn default() -> Self {
        Self {
            from: Default::default(),
            asset_id: BASE_ASSET_ID,
            amount: Default::default(),
            excluded_utxos: Default::default(),
            excluded_message_nonces: Default::default(),
        }
    }
}

#[derive(Debug, Error)]
pub enum ProviderError {
    // Every IO error in the context of Provider comes from the gql client
    #[error("Client request error: {0}")]
    ClientRequestError(#[from] io::Error),
    #[error("Receipts have not yet been propagated. Retry the request later.")]
    ReceiptsNotPropagatedYet,
    #[error("Invalid Fuel client version: {0}")]
    InvalidFuelClientVersion(#[from] semver::Error),
    #[error("Unsupported Fuel client version. Current version: {current}, supported version: {supported}")]
    UnsupportedFuelClientVersion {
        current: semver::Version,
        supported: semver::Version,
    },
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
        let client = RetryableClient::new(&url, Default::default())?;
        let consensus_parameters = client.chain_info().await?.consensus_parameters;
        let node_info = client.node_info().await?.into();

        Self::ensure_client_version_is_supported(&node_info)?;

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
    pub async fn send_transaction_and_await_commit<T: Transaction>(&self, tx: T) -> Result<TxId> {
        let tx_id = self.send_transaction(tx.clone()).await?;
        let _status = self.client.await_transaction_commit(&tx_id).await?;

        #[cfg(feature = "coin-cache")]
        {
            if matches!(
                _status,
                TransactionStatus::SqueezedOut { .. } | TransactionStatus::Failure { .. }
            ) {
                self.cache.lock().await.remove_items(tx.used_coins())
            }
        }

        Ok(tx_id)
    }

    pub async fn send_transaction<T: Transaction>(&self, mut tx: T) -> Result<TxId> {
        tx.precompute(&self.chain_id())?;

        let chain_info = self.chain_info().await?;
        tx.check_without_signatures(
            chain_info.latest_block.header.height,
            self.consensus_parameters(),
        )?;

        if tx.is_using_predicates() {
            tx.estimate_predicates(&self.consensus_parameters)?;
        }

        self.validate_transaction(tx.clone()).await?;

        self.submit(tx).await
    }

    async fn validate_transaction<T: Transaction>(&self, tx: T) -> Result<()> {
        let tolerance = 0.0;
        let TransactionCost {
            gas_used,
            min_gas_price,
            ..
        } = self
            .estimate_transaction_cost(tx.clone(), Some(tolerance))
            .await?;

        tx.validate_gas(min_gas_price, gas_used)?;

        Ok(())
    }

    #[cfg(not(feature = "coin-cache"))]
    async fn submit<T: Transaction>(&self, tx: T) -> Result<TxId> {
        Ok(self.client.submit(&tx.into()).await?)
    }

    #[cfg(feature = "coin-cache")]
    async fn submit<T: Transaction>(&self, tx: T) -> Result<TxId> {
        let used_utxos = tx.used_coins();
        let tx_id = self.client.submit(&tx.into()).await?;
        self.cache.lock().await.insert_multiple(used_utxos);

        Ok(tx_id)
    }

    pub async fn tx_status(&self, tx_id: &TxId) -> ProviderResult<TxStatus> {
        self.client
            .transaction_status(tx_id)
            .await
            .map(Into::into)
            .map_err(Into::into)
    }

    pub async fn chain_info(&self) -> ProviderResult<ChainInfo> {
        Ok(self.client.chain_info().await?.into())
    }

    pub fn consensus_parameters(&self) -> &ConsensusParameters {
        &self.consensus_parameters
    }

    fn ensure_client_version_is_supported(node_info: &NodeInfo) -> ProviderResult<()> {
        let node_version = node_info.node_version.parse::<semver::Version>()?;
        let VersionCompatibility {
            supported_version,
            is_major_supported,
            is_minor_supported,
            is_patch_supported,
        } = check_fuel_core_version_compatibility(node_version.clone());

        if !is_major_supported || !is_minor_supported {
            return Err(ProviderError::UnsupportedFuelClientVersion {
                current: node_version,
                supported: supported_version,
            });
        } else if !is_patch_supported {
            tracing::warn!(
                fuel_client_version = %node_version,
                supported_version = %supported_version,
                "The patch versions of the client and SDK differ.",
            );
        };

        Ok(())
    }

    pub fn chain_id(&self) -> ChainId {
        self.consensus_parameters.chain_id
    }

    pub async fn node_info(&self) -> ProviderResult<NodeInfo> {
        Ok(self.client.node_info().await?.into())
    }

    pub async fn checked_dry_run<T: Transaction>(&self, tx: T) -> Result<TxStatus> {
        let receipts = self.dry_run(tx).await?;
        Ok(Self::tx_status_from_receipts(receipts))
    }

    fn tx_status_from_receipts(receipts: Vec<Receipt>) -> TxStatus {
        let revert_reason = receipts.iter().find_map(|receipt| match receipt {
            Receipt::ScriptResult { result, .. } if *result != ScriptExecutionResult::Success => {
                Some(format!("{result:?}"))
            }
            _ => None,
        });

        match revert_reason {
            Some(reason) => TxStatus::Revert {
                receipts,
                reason,
                revert_id: 0,
            },
            None => TxStatus::Success { receipts },
        }
    }

    pub async fn dry_run<T: Transaction>(&self, tx: T) -> Result<Vec<Receipt>> {
        let receipts = self.client.dry_run(&tx.into()).await?;

        Ok(receipts)
    }

    pub async fn dry_run_no_validation<T: Transaction>(&self, tx: T) -> Result<Vec<Receipt>> {
        let receipts = self.client.dry_run_opt(&tx.into(), Some(false)).await?;

        Ok(receipts)
    }

    /// Gets all unspent coins owned by address `from`, with asset ID `asset_id`.
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

    async fn request_coins_to_spend(
        &self,
        filter: ResourceFilter,
    ) -> ProviderResult<Vec<CoinType>> {
        let queries = filter.resource_queries();

        let res = self
            .client
            .coins_to_spend(
                &filter.owner(),
                queries.spend_query(),
                queries.exclusion_query(),
            )
            .await?
            .into_iter()
            .flatten()
            .map(|c| CoinType::try_from(c).map_err(ProviderError::ClientRequestError))
            .collect::<ProviderResult<Vec<CoinType>>>()?;

        Ok(res)
    }

    /// Get some spendable coins of asset `asset_id` for address `from` that add up at least to
    /// amount `amount`. The returned coins (UTXOs) are actual coins that can be spent. The number
    /// of coins (UXTOs) is optimized to prevent dust accumulation.
    #[cfg(not(feature = "coin-cache"))]
    pub async fn get_spendable_resources(
        &self,
        filter: ResourceFilter,
    ) -> ProviderResult<Vec<CoinType>> {
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
    ) -> ProviderResult<Vec<CoinType>> {
        self.extend_filter_with_cached(&mut filter).await;
        self.request_coins_to_spend(filter).await
    }

    #[cfg(feature = "coin-cache")]
    async fn extend_filter_with_cached(&self, filter: &mut ResourceFilter) {
        let mut cache = self.cache.lock().await;
        let used_coins = cache.get_active(&(filter.from.clone(), filter.asset_id));

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
    ) -> ProviderResult<u64> {
        self.client
            .balance(&address.into(), Some(&asset_id))
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
            .contract_balance(&contract_id.into(), Some(&asset_id))
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
    ) -> ProviderResult<HashMap<AssetId, u64>> {
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

    pub async fn get_transaction_by_id(
        &self,
        tx_id: &TxId,
    ) -> ProviderResult<Option<TransactionResponse>> {
        Ok(self.client.transaction(tx_id).await?.map(Into::into))
    }

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
            .transactions_by_owner(&owner.into(), request)
            .await?;

        Ok(PaginatedResult {
            cursor: pr.cursor,
            results: pr.results.into_iter().map(Into::into).collect(),
            has_next_page: pr.has_next_page,
            has_previous_page: pr.has_previous_page,
        })
    }

    pub async fn latest_block_height(&self) -> ProviderResult<u32> {
        Ok(self.chain_info().await?.latest_block.header.height)
    }

    pub async fn latest_block_time(&self) -> ProviderResult<Option<DateTime<Utc>>> {
        Ok(self.chain_info().await?.latest_block.header.time)
    }

    pub async fn produce_blocks(
        &self,
        blocks_to_produce: u32,
        start_time: Option<DateTime<Utc>>,
    ) -> io::Result<u32> {
        let start_time = start_time.map(|time| Tai64::from_unix(time.timestamp()).0);
        self.client
            .produce_blocks(blocks_to_produce, start_time)
            .await
            .map(Into::into)
    }

    /// Get block by id.
    pub async fn block(&self, block_id: &Bytes32) -> ProviderResult<Option<Block>> {
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

    pub async fn estimate_transaction_cost<T: Transaction>(
        &self,
        tx: T,
        tolerance: Option<f64>,
    ) -> Result<TransactionCost> {
        let NodeInfo { min_gas_price, .. } = self.node_info().await?;
        let gas_price = std::cmp::max(tx.gas_price(), min_gas_price);
        let tolerance = tolerance.unwrap_or(DEFAULT_GAS_ESTIMATION_TOLERANCE);

        let gas_used = self
            .get_gas_used_with_tolerance(tx.clone(), tolerance)
            .await?;

        let transaction_fee = tx
            .clone()
            .fee_checked_from_tx(&self.consensus_parameters)
            .expect("Error calculating TransactionFee");

        Ok(TransactionCost {
            min_gas_price,
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
    ) -> ProviderResult<Option<MessageProof>> {
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
        let receipts = self.client.dry_run_opt(&tx, Some(false)).await?;
        let gas_used = self.get_gas_used(&receipts);
        Ok((gas_used as f64 * (1.0 + tolerance as f64)) as u64)
    }

    async fn min_gas_price(&self) -> Result<u64> {
        self.node_info()
            .await
            .map(|ni| ni.min_gas_price)
            .map_err(Into::into)
    }

    fn consensus_parameters(&self) -> &ConsensusParameters {
        self.consensus_parameters()
    }
}
