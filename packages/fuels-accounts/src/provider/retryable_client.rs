use std::{future::Future, io};

use fuel_core_client::client::{
    pagination::{PaginatedResult, PaginationRequest},
    types,
    types::{primitives::BlockId, TransactionResponse, TransactionStatus},
    FuelClient,
};
use fuel_tx::{Receipt, Transaction, TxId, UtxoId};
use fuel_types::{Address, AssetId, BlockHeight, ContractId, MessageId, Nonce};
use fuels_core::{
    error,
    types::errors::{Error, Result},
};

use crate::provider::{retry_util, RetryConfig};

#[derive(Debug, Clone)]
pub(crate) struct RetryableClient {
    client: FuelClient,
    url: String,
    retry_config: RetryConfig,
}

impl RetryableClient {
    pub(crate) fn new(url: impl AsRef<str>, retry_config: RetryConfig) -> Result<Self> {
        let url = url.as_ref().to_string();
        let client = FuelClient::new(&url).map_err(|err| error!(InfrastructureError, "{err}"))?;
        Ok(Self {
            client,
            retry_config,
            url,
        })
    }

    pub(crate) fn url(&self) -> &str {
        &self.url
    }

    pub(crate) fn set_retry_config(&mut self, retry_config: RetryConfig) {
        self.retry_config = retry_config;
    }

    async fn our_retry<T, Fut>(&self, action: impl Fn() -> Fut) -> io::Result<T>
    where
        Fut: Future<Output = io::Result<T>>,
    {
        retry_util::retry(action, &self.retry_config, |result| result.is_err()).await
    }

    // DELEGATION START
    pub async fn health(&self) -> io::Result<bool> {
        self.our_retry(|| self.client.health()).await
    }

    pub async fn transaction(&self, id: &TxId) -> io::Result<Option<TransactionResponse>> {
        self.our_retry(|| self.client.transaction(id)).await
    }

    pub(crate) async fn chain_info(&self) -> io::Result<types::ChainInfo> {
        self.our_retry(|| self.client.chain_info()).await
    }

    pub async fn await_transaction_commit(&self, id: &TxId) -> io::Result<TransactionStatus> {
        self.our_retry(|| self.client.await_transaction_commit(id))
            .await
    }

    pub async fn submit(&self, tx: &Transaction) -> io::Result<types::primitives::TransactionId> {
        self.our_retry(|| self.client.submit(tx)).await
    }

    pub async fn receipts(&self, id: &TxId) -> io::Result<Option<Vec<Receipt>>> {
        retry_util::retry(
            || self.client.receipts(id),
            &self.retry_config,
            |result| !matches!(result, Ok(Some(_))),
        )
        .await
    }

    pub async fn transaction_status(&self, id: &TxId) -> io::Result<TransactionStatus> {
        self.our_retry(|| self.client.transaction_status(id)).await
    }

    pub async fn node_info(&self) -> io::Result<types::NodeInfo> {
        self.our_retry(|| self.client.node_info()).await
    }
    pub async fn dry_run(&self, tx: &Transaction) -> io::Result<Vec<Receipt>> {
        self.our_retry(|| self.client.dry_run(tx)).await
    }

    pub async fn dry_run_opt(
        &self,
        tx: &Transaction,
        utxo_validation: Option<bool>,
    ) -> io::Result<Vec<Receipt>> {
        self.our_retry(|| self.client.dry_run_opt(tx, utxo_validation))
            .await
    }

    pub async fn coins(
        &self,
        owner: &Address,
        asset_id: Option<&AssetId>,
        request: PaginationRequest<String>,
    ) -> io::Result<PaginatedResult<types::Coin, String>> {
        self.our_retry(move || self.client.coins(owner, asset_id, request.clone()))
            .await
    }

    pub async fn coins_to_spend(
        &self,
        owner: &Address,
        spend_query: Vec<(AssetId, u64, Option<u64>)>,
        excluded_ids: Option<(Vec<UtxoId>, Vec<Nonce>)>,
    ) -> io::Result<Vec<Vec<types::CoinType>>> {
        self.client
            .coins_to_spend(owner, spend_query, excluded_ids)
            .await
    }

    pub async fn balance(&self, owner: &Address, asset_id: Option<&AssetId>) -> io::Result<u64> {
        self.our_retry(|| self.client.balance(owner, asset_id))
            .await
    }

    pub async fn contract_balance(
        &self,
        id: &ContractId,
        asset: Option<&AssetId>,
    ) -> io::Result<u64> {
        self.our_retry(|| self.client.contract_balance(id, asset))
            .await
    }

    pub async fn contract_balances(
        &self,
        contract: &ContractId,
        request: PaginationRequest<String>,
    ) -> io::Result<PaginatedResult<types::ContractBalance, String>> {
        self.our_retry(|| self.client.contract_balances(contract, request.clone()))
            .await
    }

    pub async fn balances(
        &self,
        owner: &Address,
        request: PaginationRequest<String>,
    ) -> io::Result<PaginatedResult<types::Balance, String>> {
        self.our_retry(|| self.client.balances(owner, request.clone()))
            .await
    }

    pub async fn transactions(
        &self,
        request: PaginationRequest<String>,
    ) -> io::Result<PaginatedResult<TransactionResponse, String>> {
        self.our_retry(|| self.client.transactions(request.clone()))
            .await
    }

    pub async fn transactions_by_owner(
        &self,
        owner: &Address,
        request: PaginationRequest<String>,
    ) -> io::Result<PaginatedResult<TransactionResponse, String>> {
        self.our_retry(|| self.client.transactions_by_owner(owner, request.clone()))
            .await
    }

    pub async fn produce_blocks(
        &self,
        blocks_to_produce: u64,
        start_timestamp: Option<u64>,
    ) -> io::Result<BlockHeight> {
        self.our_retry(|| {
            self.client
                .produce_blocks(blocks_to_produce, start_timestamp)
        })
        .await
    }

    pub async fn block(&self, id: &BlockId) -> io::Result<Option<types::Block>> {
        self.our_retry(|| self.client.block(id)).await
    }

    pub async fn blocks(
        &self,
        request: PaginationRequest<String>,
    ) -> io::Result<PaginatedResult<types::Block, String>> {
        self.our_retry(|| self.client.blocks(request.clone())).await
    }

    pub async fn messages(
        &self,
        owner: Option<&Address>,
        request: PaginationRequest<String>,
    ) -> io::Result<PaginatedResult<types::Message, String>> {
        self.our_retry(|| self.client.messages(owner, request.clone()))
            .await
    }

    /// Request a merkle proof of an output message.
    pub async fn message_proof(
        &self,
        transaction_id: &TxId,
        message_id: &MessageId,
        commit_block_id: Option<&BlockId>,
        commit_block_height: Option<BlockHeight>,
    ) -> io::Result<Option<types::MessageProof>> {
        self.our_retry(|| {
            self.client.message_proof(
                transaction_id,
                message_id,
                commit_block_id,
                commit_block_height,
            )
        })
        .await
    }
    // DELEGATION END
}
