use anyhow::Result;

use fuel_gql_client::fuel_tx::{Input, Output, Receipt, Transaction};
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

/// Script provides methods to create and a call/simulate a
/// script transaction that carries out contract method calls
#[derive(Debug)]
pub struct Script {
    pub tx: fuels_core::tx::Script,
}

impl Script {
    pub fn new(tx: fuels_core::tx::Script) -> Self {
        Self { tx }
    }

    /// Creates a script from the script binary.
    pub fn from_binary(
        script_binary: Vec<u8>,
        tx_params: TxParameters,
        script_data: Option<Vec<u8>>,
        inputs: Option<Vec<Input>>,
        outputs: Option<Vec<Output>>,
    ) -> Self {
        let tx = Transaction::script(
            tx_params.gas_price,
            tx_params.gas_limit,
            tx_params.maturity,
            script_binary, // Pass the compiled script into the tx
            script_data.unwrap_or_default(),
            inputs.unwrap_or_default(),
            outputs.unwrap_or_default(),
            vec![vec![].into()],
        );

        Self::new(tx)
    }

    /// Creates a script from the binary located at `binary_filepath`.
    pub fn from_binary_filepath(
        binary_filepath: &str,
        tx_params: Option<TxParameters>,
        script_data: Option<Vec<u8>>,
        inputs: Option<Vec<Input>>,
        outputs: Option<Vec<Output>>,
    ) -> Result<Self, Error> {
        let script_binary = std::fs::read(binary_filepath)?;
        Ok(Script::from_binary(
            script_binary,
            tx_params.unwrap_or_default(),
            script_data,
            inputs,
            outputs,
        ))
    }

    /// Creates a Script from a contract call. The internal Transaction is initialized
    /// with the actual script instructions, script data needed to perform the call
    /// and transaction inputs/outputs consisting of assets and contracts
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

        Ok(Script::new(tx))
    }

    /// Execute the transaction in a state-modifying manner.
    pub async fn call(&self, provider: &Provider) -> Result<Vec<Receipt>, Error> {
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
