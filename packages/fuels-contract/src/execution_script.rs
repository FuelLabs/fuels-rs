use crate::contract_calls_utils::{
    extract_message_outputs, extract_unique_contract_ids, extract_variable_outputs,
    get_single_call_instructions, CallOpcodeParamsOffset,
};
use anyhow::Result;
use std::collections::HashSet;
use std::fmt::Debug;

use fuel_gql_client::fuel_tx::{Receipt, Transaction};

use fuel_tx::{AssetId, Checkable, Input, Output, ScriptExecutionResult};
use fuels_core::{offsets::call_script_data_offset, parameters::TxParameters};
use fuels_signers::provider::Provider;
use fuels_signers::{Signer, WalletUnlocked};

use fuel_tx::field::Inputs;
use fuels_core::tx::ContractId;
use fuels_types::errors::Error;
use std::vec;

use crate::contract::ContractCall;
use crate::contract_calls_utils::{
    build_script_data_from_contract_calls, calculate_required_asset_amounts, get_instructions,
    prepare_script_inputs_outputs,
};

/// [`TransactionExecution`] provides methods to create and call/simulate a transaction that carries
/// out contract method calls or script calls
#[derive(Debug)]
pub struct ExecutableFuelCall {
    pub tx: fuels_core::tx::Script,
}

#[derive(Debug)]
pub struct PrepareFuelCall {
    pub(crate) tx: fuels_core::tx::Script,
    pub(crate) calls: Calls,
    pub(crate) wallet: WalletUnlocked,
    pub(crate) inputs: Vec<Input>,
    pub(crate) outputs: Vec<Output>,
}

#[derive(Debug, Default)]
pub struct Calls {
    pub required_asset_amounts: Vec<(AssetId, u64)>,
    pub calls_contract_ids: HashSet<ContractId>,
    pub calls_variable_outputs: Vec<Output>,
    pub calls_message_outputs: Vec<Output>,
}

impl PrepareFuelCall {
    pub fn new(tx: fuels_core::tx::Script, calls: Calls, wallet: WalletUnlocked) -> Self {
        Self {
            tx,
            calls,
            wallet,
            inputs: vec![],
            outputs: vec![],
        }
    }

    /// Creates a [`TransactionExecution`] from contract calls. The internal [`Transaction`] is
    /// initialized with the actual script instructions, script data needed to perform the call and
    /// transaction inputs/outputs consisting of assets and contracts.
    pub async fn from_contract_calls(
        calls: &[ContractCall],
        tx_parameters: &TxParameters,
        wallet: &WalletUnlocked,
    ) -> Result<Self, Error> {
        let consensus_parameters = wallet.get_provider()?.consensus_parameters().await?;

        // Calculate instructions length for call instructions
        // Use placeholder for call param offsets, we only care about the length
        let calls_instructions_len =
            get_single_call_instructions(&CallOpcodeParamsOffset::default()).len() * calls.len();

        let data_offset = call_script_data_offset(&consensus_parameters, calls_instructions_len);

        let (script_data, call_param_offsets) =
            build_script_data_from_contract_calls(calls, data_offset, tx_parameters.gas_limit);

        let script = get_instructions(calls, call_param_offsets);

        let tx = Transaction::script(
            tx_parameters.gas_price,
            tx_parameters.gas_limit,
            tx_parameters.maturity,
            script,
            script_data,
            vec![],
            vec![],
            vec![],
        );

        let calls = Calls {
            required_asset_amounts: calculate_required_asset_amounts(calls),
            calls_contract_ids: extract_unique_contract_ids(calls),
            calls_variable_outputs: extract_variable_outputs(calls),
            calls_message_outputs: extract_message_outputs(calls),
        };

        Ok(PrepareFuelCall::new(tx, calls, wallet.clone()))
    }

    pub async fn prepare(&mut self) -> Result<ExecutableFuelCall, Error> {
        prepare_script_inputs_outputs(self).await?;
        Ok(ExecutableFuelCall::new(self.tx.clone()))
    }

    pub fn add_inputs(&mut self, inputs: Vec<Input>) -> &mut PrepareFuelCall {
        self.inputs.extend(inputs);
        self
    }

    pub fn add_outputs(&mut self, outputs: Vec<Output>) -> &mut PrepareFuelCall {
        self.outputs.extend(outputs);
        self
    }
}

impl ExecutableFuelCall {
    pub fn new(tx: fuels_core::tx::Script) -> Self {
        Self { tx }
    }

    /// Execute the transaction in a state-modifying manner.
    pub async fn execute(&self, provider: &Provider) -> Result<Vec<Receipt>, Error> {
        let chain_info = provider.chain_info().await?;

        self.tx.check_without_signatures(
            chain_info.latest_block.header.height,
            &chain_info.consensus_parameters,
        )?;

        provider.send_transaction(&self.tx).await
    }

    /// Execute the transaction in a simulated manner, not modifying blockchain state
    pub async fn simulate(&self, provider: &Provider) -> Result<Vec<Receipt>, Error> {
        let chain_info = provider.chain_info().await?;

        self.tx.check_without_signatures(
            chain_info.latest_block.header.height,
            &chain_info.consensus_parameters,
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
