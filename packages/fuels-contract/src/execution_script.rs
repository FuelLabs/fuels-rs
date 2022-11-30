use anyhow::Result;
use std::fmt::Debug;

use fuel_gql_client::fuel_tx::{Receipt, Transaction};
use fuel_gql_client::fuel_types::AssetId;

use fuel_tx::{Checkable, ScriptExecutionResult};
use fuels_core::parameters::TxParameters;
use fuels_signers::provider::Provider;
use fuels_signers::{Signer, WalletUnlocked};

use fuels_types::errors::Error;

use std::vec;

use crate::contract::ContractCall;
use crate::contract_calls_utils::{
    build_script_data_from_contract_calls, calculate_required_asset_amounts, get_data_offset,
    get_instructions, get_transaction_inputs_outputs,
};

/// [`TransactionExecution`] provides methods to create and call/simulate a transaction that carries
/// out contract method calls or script calls
#[derive(Debug)]
pub struct ExecutableFuelCall {
    pub tx: fuels_core::tx::Script,
}

impl ExecutableFuelCall {
    pub fn new(tx: fuels_core::tx::Script) -> Self {
        Self { tx }
    }

    /// Creates a [`TransactionExecution`] from contract calls. The internal [`Transaction`] is
    /// initialized with the actual script instructions, script data needed to perform the call and
    /// transaction inputs/outputs consisting of assets and contracts.
    pub async fn from_contract_calls(
        calls: &[ContractCall],
        tx_parameters: &TxParameters,
        wallet: &WalletUnlocked,
    ) -> Result<Self, Error> {
        let data_offset = get_data_offset(calls.len());

        let (script_data, call_param_offsets) =
            build_script_data_from_contract_calls(calls, data_offset, tx_parameters.gas_limit);

        let script = get_instructions(calls, call_param_offsets);

        let required_asset_amounts = calculate_required_asset_amounts(calls);
        let mut spendable_resources = vec![];

        // Find the spendable resources required for those calls
        for (asset_id, amount) in &required_asset_amounts {
            let resources = wallet.get_spendable_resources(*asset_id, *amount).await?;
            spendable_resources.extend(resources);
        }

        let (inputs, outputs) =
            get_transaction_inputs_outputs(calls, wallet.address(), spendable_resources);

        let mut tx = Transaction::script(
            tx_parameters.gas_price,
            tx_parameters.gas_limit,
            tx_parameters.maturity,
            script,
            script_data,
            inputs,
            outputs,
            vec![],
        );

        let base_asset_amount = required_asset_amounts
            .iter()
            .find(|(asset_id, _)| *asset_id == AssetId::default());
        match base_asset_amount {
            Some((_, base_amount)) => wallet.add_fee_coins(&mut tx, *base_amount, 0).await?,
            None => wallet.add_fee_coins(&mut tx, 0, 0).await?,
        }
        wallet.sign_transaction(&mut tx).await.unwrap();

        Ok(ExecutableFuelCall::new(tx))
    }

    /// Execute the transaction in a state-modifying manner.
    pub async fn execute(&self, provider: &Provider) -> Result<Vec<Receipt>, Error> {
        let chain_info = provider.chain_info().await?;

        self.tx.check_without_signatures(
            chain_info.latest_block.header.height.0,
            &chain_info.consensus_parameters.into(),
        )?;

        provider.send_transaction(&self.tx).await
    }

    /// Execute the transaction in a simulated manner, not modifying blockchain state
    pub async fn simulate(&self, provider: &Provider) -> Result<Vec<Receipt>, Error> {
        let chain_info = provider.chain_info().await?;

        self.tx.check_without_signatures(
            chain_info.latest_block.header.height.0,
            &chain_info.consensus_parameters.into(),
        )?;

        let receipts = provider.dry_run(&self.tx.clone().into()).await?;
        if receipts
            .iter()
            .any(|r|
                matches!(r, Receipt::ScriptResult { result, .. } if *result != ScriptExecutionResult::Success)
        ) {
            return Err(Error::RevertTransactionError(Default::default(), receipts));
        }

        Ok(receipts)
    }
}
