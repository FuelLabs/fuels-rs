use std::{collections::HashMap, fmt::Debug, io};

use chrono::{DateTime, Utc};
#[cfg(feature = "fuel-core")]
use fuel_core::service::{Config, FuelService};
use fuel_core_client::client::{
    schema::{balance::Balance, contract::ContractBalance},
    types::TransactionStatus,
    FuelClient, PageDirection, PaginatedResult, PaginationRequest,
};
use fuel_tx::{AssetId, ConsensusParameters, Receipt, ScriptExecutionResult, UtxoId};
use fuel_types::Nonce;
use fuel_vm::state::ProgramState;
use fuels_core::{
    constants::{BASE_ASSET_ID, DEFAULT_GAS_ESTIMATION_TOLERANCE, MAX_GAS_PER_TX},
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
        transaction_response::TransactionResponse,
    },
};
use itertools::Itertools;
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

pub(crate) struct ResourceQueries {
    utxos: Vec<String>,
    messages: Vec<String>,
    asset_id: String,
    amount: u64,
}

impl ResourceQueries {
    pub fn new(
        utxo_ids: Vec<UtxoId>,
        message_nonces: Vec<Nonce>,
        asset_id: AssetId,
        amount: u64,
    ) -> Self {
        let utxos = utxo_ids
            .iter()
            .map(|utxo_id| format!("{utxo_id:#x}"))
            .collect::<Vec<_>>();

        let messages = message_nonces
            .iter()
            .map(|nonce| format!("{nonce:#x}"))
            .collect::<Vec<_>>();

        Self {
            utxos,
            messages,
            asset_id: format!("{asset_id:#x}"),
            amount,
        }
    }

    pub fn exclusion_query(&self) -> Option<(Vec<&str>, Vec<&str>)> {
        if self.utxos.is_empty() && self.messages.is_empty() {
            return None;
        }

        let utxos_as_str = self.utxos.iter().map(AsRef::as_ref).collect::<Vec<_>>();

        let msg_ids_as_str = self.messages.iter().map(AsRef::as_ref).collect::<Vec<_>>();

        Some((utxos_as_str, msg_ids_as_str))
    }

    pub fn spend_query(&self) -> Vec<(&str, u64, Option<u64>)> {
        vec![(self.asset_id.as_str(), self.amount, None)]
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
    pub fn owner(&self) -> String {
        self.from.hash().to_string()
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
    #[error(transparent)]
    ClientRequestError(#[from] io::Error),
}

impl From<ProviderError> for Error {
    fn from(e: ProviderError) -> Self {
        Error::ProviderError(e.to_string())
    }
}

/// Extends the functionality of the [`FuelClient`].
#[async_trait::async_trait]
pub trait ClientExt {
    // TODO: It should be part of the `fuel-core-client`. See https://github.com/FuelLabs/fuel-core/issues/1178
    /// Submits transaction, await confirmation and return receipts.
    async fn submit_and_await_commit_with_receipts(
        &self,
        tx: &fuel_tx::Transaction,
    ) -> io::Result<(TransactionStatus, Option<Vec<Receipt>>)>;
}

#[async_trait::async_trait]
impl ClientExt for FuelClient {
    async fn submit_and_await_commit_with_receipts(
        &self,
        tx: &fuel_tx::Transaction,
    ) -> io::Result<(TransactionStatus, Option<Vec<Receipt>>)> {
        let tx_id = self.submit(tx).await?.to_string();
        let status = self.await_transaction_commit(&tx_id).await?;
        let receipts = self.receipts(&tx_id).await?;

        Ok((status, receipts))
    }
}

/// Encapsulates common client operations in the SDK.
/// Note that you may also use `client`, which is an instance
/// of `FuelClient`, directly, which provides a broader API.
#[derive(Debug, Clone)]
pub struct Provider {
    pub client: FuelClient,
    pub consensus_parameters: ConsensusParameters,
}

impl Provider {
    pub fn new(client: FuelClient, consensus_parameters: ConsensusParameters) -> Self {
        Self {
            client,
            consensus_parameters,
        }
    }

    /// Sends a transaction to the underlying Provider's client.
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
        let receipts = receipts.map_or(vec![], |v| v);
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
    ) -> ProviderResult<(TransactionStatus, Option<Vec<Receipt>>)> {
        self.client
            .submit_and_await_commit_with_receipts(&tx.into())
            .await
            .map_err(Into::into)
    }

    #[cfg(feature = "fuel-core")]
    /// Launches a local `fuel-core` network based on provided config.
    pub async fn launch(config: Config) -> Result<FuelClient> {
        let srv = FuelService::new_node(config).await.unwrap();
        Ok(FuelClient::from(srv.bound_address))
    }

    /// Connects to an existing node at the given address.
    pub async fn connect(url: impl AsRef<str>) -> Result<Provider> {
        let client = FuelClient::new(url).map_err(|err| error!(InfrastructureError, "{err}"))?;
        let consensus_parameters = client.chain_info().await?.consensus_parameters.into();
        Ok(Provider::new(client, consensus_parameters))
    }

    pub async fn chain_info(&self) -> ProviderResult<ChainInfo> {
        Ok(self.client.chain_info().await?.into())
    }

    pub fn consensus_parameters(&self) -> ConsensusParameters {
        self.consensus_parameters
    }

    pub async fn node_info(&self) -> ProviderResult<NodeInfo> {
        Ok(self.client.node_info().await?.into())
    }

    pub async fn checked_dry_run<T: Transaction + Clone>(&self, tx: &T) -> Result<Vec<Receipt>> {
        let receipts = self.dry_run(tx).await?;
        Self::has_script_succeeded(&receipts)?;

        Ok(receipts)
    }

    fn has_script_succeeded(receipts: &[Receipt]) -> Result<()> {
        receipts
            .iter()
            .find_map(|receipt| match receipt {
                Receipt::ScriptResult { result, .. }
                    if *result != ScriptExecutionResult::Success =>
                {
                    Some(format!("{result:?}"))
                }
                _ => None,
            })
            .map(|error_message| {
                Err(Error::RevertTransactionError {
                    reason: error_message,
                    revert_id: 0,
                    receipts: receipts.to_owned(),
                })
            })
            .unwrap_or(Ok(()))
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
            .map(|coin_type| {
                coin_type
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
        commit_block_id: Option<&str>,
        commit_block_height: Option<u32>,
    ) -> ProviderResult<Option<MessageProof>> {
        let proof = self
            .client
            .message_proof(
                tx_id,
                message_id,
                commit_block_id,
                commit_block_height.map(Into::into),
            )
            .await?
            .map(Into::into);
        Ok(proof)
    }
}
