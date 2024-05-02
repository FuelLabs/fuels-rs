use std::{future::Future, io};

use fuel_core_client::client::{
    pagination::{PaginatedResult, PaginationRequest},
    types::{
        gas_price::{EstimateGasPrice, LatestGasPrice},
        primitives::{BlockId, TransactionId},
        Balance, Block, ChainInfo, Coin, CoinType, ContractBalance, Message, MessageProof,
        NodeInfo, TransactionResponse, TransactionStatus,
    },
    FuelClient,
};
use fuel_core_types::services::executor::TransactionExecutionStatus;
use fuel_tx::{Transaction, TxId, UtxoId};
use fuel_types::{Address, AssetId, BlockHeight, ContractId, Nonce};
use fuels_core::types::errors::{error, Error, Result};

use super::supported_versions::{self, VersionCompatibility};
use crate::provider::{retry_util, RetryConfig};

#[derive(Debug, thiserror::Error)]
pub(crate) enum RequestError {
    #[error("io error: {0}")]
    IO(String),
}

type RequestResult<T> = std::result::Result<T, RequestError>;

impl From<RequestError> for Error {
    fn from(e: RequestError) -> Self {
        Error::Provider(e.to_string())
    }
}

#[derive(Debug, Clone)]
pub(crate) struct RetryableClient {
    client: FuelClient,
    url: String,
    retry_config: RetryConfig,
    prepend_warning: Option<String>,
}

impl RetryableClient {
    pub(crate) async fn connect(url: impl AsRef<str>, retry_config: RetryConfig) -> Result<Self> {
        let url = url.as_ref().to_string();
        let client = FuelClient::new(&url).map_err(|e| error!(Provider, "{e}"))?;

        let node_info = client.node_info().await?;
        let warning = Self::version_compatibility_warning(&node_info)?;

        Ok(Self {
            client,
            retry_config,
            url,
            prepend_warning: warning,
        })
    }

    fn version_compatibility_warning(node_info: &NodeInfo) -> Result<Option<String>> {
        let node_version = node_info
            .node_version
            .parse::<semver::Version>()
            .map_err(|e| error!(Provider, "could not parse Fuel client version: {}", e))?;

        let VersionCompatibility {
            supported_version,
            is_major_supported,
            is_minor_supported,
            ..
        } = supported_versions::compare_node_compatibility(node_version.clone());

        let msg = if !is_major_supported || !is_minor_supported {
            Some(format!(
                "warning: the fuel node version to which this provider is connected has a semver incompatible version from the one the SDK was developed against. Connected node version: {node_version}, supported version: {supported_version}",
            ))
        } else {
            None
        };

        Ok(msg)
    }

    pub(crate) fn url(&self) -> &str {
        &self.url
    }

    pub(crate) fn set_retry_config(&mut self, retry_config: RetryConfig) {
        self.retry_config = retry_config;
    }

    async fn wrap<T, Fut>(&self, action: impl Fn() -> Fut) -> RequestResult<T>
    where
        Fut: Future<Output = io::Result<T>>,
    {
        retry_util::retry(action, &self.retry_config, |result| result.is_err())
            .await
            .map_err(|e| {
                let msg = if let Some(warning) = &self.prepend_warning {
                    format!("{warning}. {e}")
                } else {
                    e.to_string()
                };
                RequestError::IO(msg)
            })
    }

    // DELEGATION START
    pub async fn health(&self) -> RequestResult<bool> {
        self.wrap(|| self.client.health()).await
    }

    pub async fn transaction(&self, id: &TxId) -> RequestResult<Option<TransactionResponse>> {
        self.wrap(|| self.client.transaction(id)).await
    }

    pub(crate) async fn chain_info(&self) -> RequestResult<ChainInfo> {
        self.wrap(|| self.client.chain_info()).await
    }

    pub async fn await_transaction_commit(&self, id: &TxId) -> RequestResult<TransactionStatus> {
        self.wrap(|| self.client.await_transaction_commit(id)).await
    }

    pub async fn submit_and_await_commit(
        &self,
        tx: &Transaction,
    ) -> RequestResult<TransactionStatus> {
        self.wrap(|| self.client.submit_and_await_commit(tx)).await
    }

    pub async fn submit(&self, tx: &Transaction) -> RequestResult<TransactionId> {
        self.wrap(|| self.client.submit(tx)).await
    }

    pub async fn transaction_status(&self, id: &TxId) -> RequestResult<TransactionStatus> {
        self.wrap(|| self.client.transaction_status(id)).await
    }

    pub async fn node_info(&self) -> RequestResult<NodeInfo> {
        self.wrap(|| self.client.node_info()).await
    }

    pub async fn latest_gas_price(&self) -> RequestResult<LatestGasPrice> {
        self.wrap(|| self.client.latest_gas_price()).await
    }

    pub async fn estimate_gas_price(&self, block_horizon: u32) -> RequestResult<EstimateGasPrice> {
        self.wrap(|| self.client.estimate_gas_price(block_horizon))
            .await
            .map(Into::into)
    }

    pub async fn dry_run(
        &self,
        tx: &[Transaction],
    ) -> RequestResult<Vec<TransactionExecutionStatus>> {
        self.wrap(|| self.client.dry_run(tx)).await
    }

    pub async fn dry_run_opt(
        &self,
        tx: &[Transaction],
        utxo_validation: Option<bool>,
    ) -> RequestResult<Vec<TransactionExecutionStatus>> {
        self.wrap(|| self.client.dry_run_opt(tx, utxo_validation))
            .await
    }

    pub async fn coins(
        &self,
        owner: &Address,
        asset_id: Option<&AssetId>,
        request: PaginationRequest<String>,
    ) -> RequestResult<PaginatedResult<Coin, String>> {
        self.wrap(move || self.client.coins(owner, asset_id, request.clone()))
            .await
    }

    pub async fn coins_to_spend(
        &self,
        owner: &Address,
        spend_query: Vec<(AssetId, u64, Option<u32>)>,
        excluded_ids: Option<(Vec<UtxoId>, Vec<Nonce>)>,
    ) -> RequestResult<Vec<Vec<CoinType>>> {
        self.wrap(move || {
            self.client
                .coins_to_spend(owner, spend_query.clone(), excluded_ids.clone())
        })
        .await
    }

    pub async fn balance(&self, owner: &Address, asset_id: Option<&AssetId>) -> RequestResult<u64> {
        self.wrap(|| self.client.balance(owner, asset_id)).await
    }

    pub async fn contract_balance(
        &self,
        id: &ContractId,
        asset: Option<&AssetId>,
    ) -> RequestResult<u64> {
        self.wrap(|| self.client.contract_balance(id, asset)).await
    }

    pub async fn contract_balances(
        &self,
        contract: &ContractId,
        request: PaginationRequest<String>,
    ) -> RequestResult<PaginatedResult<ContractBalance, String>> {
        self.wrap(|| self.client.contract_balances(contract, request.clone()))
            .await
    }

    pub async fn balances(
        &self,
        owner: &Address,
        request: PaginationRequest<String>,
    ) -> RequestResult<PaginatedResult<Balance, String>> {
        self.wrap(|| self.client.balances(owner, request.clone()))
            .await
    }

    pub async fn transactions(
        &self,
        request: PaginationRequest<String>,
    ) -> RequestResult<PaginatedResult<TransactionResponse, String>> {
        self.wrap(|| self.client.transactions(request.clone()))
            .await
    }

    pub async fn transactions_by_owner(
        &self,
        owner: &Address,
        request: PaginationRequest<String>,
    ) -> RequestResult<PaginatedResult<TransactionResponse, String>> {
        self.wrap(|| self.client.transactions_by_owner(owner, request.clone()))
            .await
    }

    pub async fn produce_blocks(
        &self,
        blocks_to_produce: u32,
        start_timestamp: Option<u64>,
    ) -> RequestResult<BlockHeight> {
        self.wrap(|| {
            self.client
                .produce_blocks(blocks_to_produce, start_timestamp)
        })
        .await
    }

    pub async fn block(&self, id: &BlockId) -> RequestResult<Option<Block>> {
        self.wrap(|| self.client.block(id)).await
    }

    pub async fn block_by_height(&self, height: BlockHeight) -> RequestResult<Option<Block>> {
        self.wrap(|| self.client.block_by_height(height)).await
    }

    pub async fn blocks(
        &self,
        request: PaginationRequest<String>,
    ) -> RequestResult<PaginatedResult<Block, String>> {
        self.wrap(|| self.client.blocks(request.clone())).await
    }

    pub async fn messages(
        &self,
        owner: Option<&Address>,
        request: PaginationRequest<String>,
    ) -> RequestResult<PaginatedResult<Message, String>> {
        self.wrap(|| self.client.messages(owner, request.clone()))
            .await
    }

    /// Request a merkle proof of an output message.
    pub async fn message_proof(
        &self,
        transaction_id: &TxId,
        nonce: &Nonce,
        commit_block_id: Option<&BlockId>,
        commit_block_height: Option<BlockHeight>,
    ) -> RequestResult<Option<MessageProof>> {
        self.wrap(|| {
            self.client
                .message_proof(transaction_id, nonce, commit_block_id, commit_block_height)
        })
        .await
    }
    // DELEGATION END
}
