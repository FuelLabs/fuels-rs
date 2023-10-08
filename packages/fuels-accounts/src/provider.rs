use std::{collections::HashMap, fmt::Debug, io, net::SocketAddr};

mod retry_util;
mod retryable_client;

use chrono::{DateTime, Utc};
use fuel_core_client::client::{
    pagination::{PageDirection, PaginatedResult, PaginationRequest},
    types::{balance::Balance, contract::ContractBalance, TransactionStatus},
};
use fuel_tx::{AssetId, ConsensusParameters, Receipt, ScriptExecutionResult, TxId, UtxoId};
use fuel_types::{Address, Bytes32, ChainId, MessageId, Nonce};
use fuel_vm::state::ProgramState;
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
        transaction_builders::NetworkInfo,
        transaction_response::TransactionResponse,
        tx_status::TxStatus,
    },
};
pub use retry_util::{Backoff, RetryConfig};
use tai64::Tai64;
use thiserror::Error;

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

    pub fn spend_query(&self) -> Vec<(AssetId, u64, Option<u64>)> {
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
}

impl Provider {
    pub fn new(url: impl AsRef<str>, consensus_parameters: ConsensusParameters) -> Result<Self> {
        let client = RetryableClient::new(&url, Default::default())?;

        Ok(Self {
            client,
            consensus_parameters,
        })
    }

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
        let consensus_parameters = client.chain_info().await?.consensus_parameters.into();

        Ok(Self {
            client,
            consensus_parameters,
        })
    }

    pub fn url(&self) -> &str {
        self.client.url()
    }

    /// Sends a transaction to the underlying Provider's client.
    pub async fn send_transaction_and_await_commit<T: Transaction>(&self, tx: T) -> Result<TxId> {
        let tx_id = self.send_transaction(tx.clone()).await?;
        self.client.await_transaction_commit(&tx_id).await?;

        Ok(tx_id)
    }

    pub async fn send_transaction<T: Transaction>(&self, tx: T) -> Result<TxId> {
        let tolerance = 0.0;
        let TransactionCost {
            gas_used,
            min_gas_price,
            ..
        } = self
            .estimate_transaction_cost(tx.clone(), Some(tolerance))
            .await?;

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
            &self.consensus_parameters(),
        )?;

        let tx_id = self.client.submit(&tx.into()).await?;
        Ok(tx_id)
    }

    pub async fn tx_status(&self, tx_id: &TxId) -> ProviderResult<TxStatus> {
        let fetch_receipts = || async {
            let receipts = self.client.receipts(tx_id).await?;
            receipts.ok_or_else(|| ProviderError::ReceiptsNotPropagatedYet)
        };

        let tx_status = self.client.transaction_status(tx_id).await?;
        let status = match tx_status {
            TransactionStatus::Success { .. } => {
                let receipts = fetch_receipts().await?;
                TxStatus::Success { receipts }
            }
            TransactionStatus::Failure {
                reason,
                program_state,
                ..
            } => {
                let receipts = fetch_receipts().await?;
                let revert_id = program_state
                    .and_then(|state| match state {
                        ProgramState::Revert(revert_id) => Some(revert_id),
                        _ => None,
                    })
                    .expect("Transaction failed without a `revert_id`");

                TxStatus::Revert {
                    receipts,
                    reason,
                    id: revert_id,
                }
            }
            TransactionStatus::Submitted { .. } => TxStatus::Submitted,
            TransactionStatus::SqueezedOut { reason } => TxStatus::SqueezedOut { reason },
        };

        Ok(status)
    }

    pub async fn chain_info(&self) -> ProviderResult<ChainInfo> {
        Ok(self.client.chain_info().await?.into())
    }

    pub fn consensus_parameters(&self) -> ConsensusParameters {
        self.consensus_parameters
    }

    pub async fn network_info(&self) -> ProviderResult<NetworkInfo> {
        let node_info = self.node_info().await?;
        let chain_info = self.chain_info().await?;

        Ok(NetworkInfo::new(node_info, chain_info))
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
                id: 0,
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

    /// Get some spendable coins of asset `asset_id` for address `from` that add up at least to
    /// amount `amount`. The returned coins (UTXOs) are actual coins that can be spent. The number
    /// of coins (UXTOs) is optimized to prevent dust accumulation.
    pub async fn get_spendable_resources(
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
        blocks_to_produce: u64,
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

        // Remove limits from an existing Transaction for accurate gas estimation
        let dry_run_tx = self.generate_dry_run_tx(tx.clone());
        let gas_used = self
            .get_gas_used_with_tolerance(dry_run_tx.clone(), tolerance)
            .await?;

        // Update the tx with estimated gas_used and correct gas price to calculate the total_fee
        let dry_run_tx = dry_run_tx
            .with_gas_price(gas_price)
            .with_gas_limit(gas_used);

        let transaction_fee = dry_run_tx
            .fee_checked_from_tx(&self.consensus_parameters)
            .expect("Error calculating TransactionFee");

        Ok(TransactionCost {
            min_gas_price,
            gas_price,
            gas_used,
            metered_bytes_size: dry_run_tx.metered_bytes_size() as u64,
            total_fee: transaction_fee.max_fee(),
        })
    }

    // Remove limits from an existing Transaction to get an accurate gas estimation
    fn generate_dry_run_tx<T: Transaction>(&self, tx: T) -> T {
        // Simulate the contract call with max gas to get the complete gas_used
        let max_gas_per_tx = self.consensus_parameters.max_gas_per_tx;
        tx.clone().with_gas_limit(max_gas_per_tx).with_gas_price(0)
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
        message_id: &MessageId,
        commit_block_id: Option<&Bytes32>,
        commit_block_height: Option<u32>,
    ) -> ProviderResult<Option<MessageProof>> {
        let proof = self
            .client
            .message_proof(
                tx_id,
                message_id,
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
