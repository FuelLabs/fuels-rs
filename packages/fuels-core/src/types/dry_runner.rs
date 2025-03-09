use std::fmt::Debug;

use async_trait::async_trait;
use fuel_core_client::client::types::assemble_tx::{AssembleTransactionResult, RequiredBalance};
use fuel_tx::{ConsensusParameters, Transaction as FuelTransaction, UtxoId};
use fuel_types::Nonce;

use crate::types::errors::Result;

#[derive(Debug, Clone, Copy)]
pub struct DryRun {
    pub succeeded: bool,
    pub script_gas: u64,
    pub variable_outputs: usize,
}

impl DryRun {
    pub fn gas_with_tolerance(&self, tolerance: f32) -> u64 {
        let gas_used = self.script_gas as f64;
        let adjusted_gas = gas_used * (1.0 + f64::from(tolerance));
        adjusted_gas.ceil() as u64
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait DryRunner: Send + Sync {
    async fn dry_run(&self, tx: FuelTransaction) -> Result<DryRun>;
    async fn estimate_gas_price(&self, block_horizon: u32) -> Result<u64>;
    async fn consensus_parameters(&self) -> Result<ConsensusParameters>;
    async fn estimate_predicates(
        &self,
        tx: &FuelTransaction,
        latest_chain_executor_version: Option<u32>,
    ) -> Result<FuelTransaction>;
    #[allow(clippy::too_many_arguments)]
    async fn assemble_tx(
        &self,
        transaction: &FuelTransaction,
        block_horizon: u32,
        required_balances: Vec<RequiredBalance>,
        fee_address_index: u16,
        exclude: Option<(Vec<UtxoId>, Vec<Nonce>)>, //TODO: exclude coins when assembling
        estimate_predicates: bool,
        reserve_gas: Option<u64>,
    ) -> Result<AssembleTransactionResult>;
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<T: DryRunner> DryRunner for &T {
    async fn dry_run(&self, tx: FuelTransaction) -> Result<DryRun> {
        (*self).dry_run(tx).await
    }

    async fn estimate_gas_price(&self, block_horizon: u32) -> Result<u64> {
        (*self).estimate_gas_price(block_horizon).await
    }

    async fn consensus_parameters(&self) -> Result<ConsensusParameters> {
        (*self).consensus_parameters().await
    }

    async fn estimate_predicates(
        &self,
        tx: &FuelTransaction,
        latest_chain_executor_version: Option<u32>,
    ) -> Result<FuelTransaction> {
        (*self)
            .estimate_predicates(tx, latest_chain_executor_version)
            .await
    }

    #[allow(clippy::too_many_arguments)]
    async fn assemble_tx(
        &self,
        transaction: &FuelTransaction,
        block_horizon: u32,
        required_balances: Vec<RequiredBalance>,
        fee_address_index: u16,
        exclude: Option<(Vec<UtxoId>, Vec<Nonce>)>, //TODO: exclude coins when assembling
        estimate_predicates: bool,
        reserve_gas: Option<u64>,
    ) -> Result<AssembleTransactionResult> {
        (*self)
            .assemble_tx(
                transaction,
                block_horizon,
                required_balances,
                fee_address_index,
                exclude,
                estimate_predicates,
                reserve_gas,
            )
            .await
    }
}
