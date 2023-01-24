use std::{fmt::Debug, vec};

use fuel_tx::{
    Address, AssetId, Bytes32, Checkable, ContractId, Input, Output, Receipt, Script,
    ScriptExecutionResult, Transaction, Witness,
};
use fuel_types::bytes::WORD_SIZE;
use fuel_vm::{
    consts::REG_ONE,
    prelude::{GTFArgs, Opcode},
};
use fuels_core::{
    constants::BASE_ASSET_ID, offsets::call_script_data_offset, parameters::TxParameters,
};
use fuels_signers::{provider::Provider, Signer, WalletUnlocked};
use fuels_types::{errors::Error, script_transaction::ScriptTransaction};

use crate::{
    call_utils::{
        build_script_data_from_contract_calls, calculate_required_asset_amounts, get_instructions,
        get_single_call_instructions, get_transaction_inputs_outputs, CallOpcodeParamsOffset,
    },
    contract::ContractCall,
};

use fuel_tx::field::{
    GasLimit, GasPrice, Inputs, Maturity, Outputs, Script as ScriptField, ScriptData, Witnesses,
};

/// [`ExecutableFuelCall`] provides methods to create and call/simulate a transaction that carries
/// out contract method calls or script calls
#[derive(Debug)]
pub struct ExecutableFuelCall {
    pub tx: ScriptTransaction,
}

impl ExecutableFuelCall {
    pub fn new(tx: ScriptTransaction) -> Self {
        Self { tx }
    }

    /*
    pub fn gas_price(&self) -> u64 {
        *self.tx.gas_price()
    }

    pub fn gas_limit(&self) -> u64 {
        self.tx.gas_limit()
    }

    pub fn maturity(&self) -> u64 {
        *self.tx.maturity()
    }

    pub fn script(&self) -> &Vec<u8> {
        self.tx.script()
    }

    pub fn script_data(&self) -> &Vec<u8> {
        self.tx.script_data()
    }

    pub fn inputs(&self) -> &Vec<Input> {
        self.tx.inputs()
    }

    pub fn outputs(&self) -> &Vec<Output> {
        self.tx.outputs()
    }

    pub fn witnesses(&self) -> &Vec<Witness> {
        self.tx.witnesses()
    }*/

    /// Creates a [`ExecutableFuelCall`] from contract calls. The internal [Transaction] is
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

        let required_asset_amounts = calculate_required_asset_amounts(calls);
        let mut spendable_resources = vec![];

        // Find the spendable resources required for those calls
        for (asset_id, amount) in &required_asset_amounts {
            let resources = wallet.get_spendable_resources(*asset_id, *amount).await?;
            spendable_resources.extend(resources);
        }

        let (inputs, outputs) =
            get_transaction_inputs_outputs(calls, wallet.address(), spendable_resources);

        let mut tx = ScriptTransaction::new(Transaction::script(
            tx_parameters.gas_price,
            tx_parameters.gas_limit,
            tx_parameters.maturity,
            script,
            script_data,
            inputs,
            outputs,
            vec![],
        ));

        let base_asset_amount = required_asset_amounts
            .iter()
            .find(|(asset_id, _)| *asset_id == AssetId::default());
        match base_asset_amount {
            Some((_, base_amount)) => wallet.add_fee_resources(&mut tx, *base_amount, 0).await?,
            None => wallet.add_fee_resources(&mut tx, 0, 0).await?,
        }
        wallet.sign_transaction(&mut tx).await.unwrap();

        Ok(ExecutableFuelCall::new(tx))
    }

    /// Craft a transaction used to transfer funds between two addresses.
    pub fn build_transfer_tx(inputs: &[Input], outputs: &[Output], params: TxParameters) -> Self {
        // This script is empty, since all this transaction does is move Inputs and Outputs around.
        let tx = Transaction::script(
            params.gas_price,
            params.gas_limit,
            params.maturity,
            vec![],
            vec![],
            inputs.to_vec(),
            outputs.to_vec(),
            vec![],
        )
        .into();

        Self { tx }
    }

    /// Craft a transaction used to transfer funds to a contract.
    pub fn build_contract_transfer_tx(
        to: ContractId,
        amount: u64,
        asset_id: AssetId,
        inputs: &[Input],
        outputs: &[Output],
        params: TxParameters,
    ) -> Self {
        let script_data: Vec<u8> = [
            to.to_vec(),
            amount.to_be_bytes().to_vec(),
            asset_id.to_vec(),
        ]
        .into_iter()
        .flatten()
        .collect();

        // This script loads:
        //  - a pointer to the contract id,
        //  - the actual amount
        //  - a pointer to the asset id
        // into the registers 0x10, 0x12, 0x13
        // and calls the TR instruction
        let script = vec![
            Opcode::gtf(0x10, 0x00, GTFArgs::ScriptData),
            Opcode::ADDI(0x11, 0x10, ContractId::LEN as u16),
            Opcode::LW(0x12, 0x11, 0),
            Opcode::ADDI(0x13, 0x11, WORD_SIZE as u16),
            Opcode::TR(0x10, 0x12, 0x13),
            Opcode::RET(REG_ONE),
        ]
        .into_iter()
        .collect();

        let tx = Transaction::script(
            params.gas_price,
            params.gas_limit,
            params.maturity,
            script,
            script_data,
            inputs.to_vec(),
            outputs.to_vec(),
            vec![],
        )
        .into();

        Self { tx }
    }

    /// Craft a transaction used to transfer funds to the base chain.
    pub fn build_message_to_output_tx(
        to: Address,
        amount: u64,
        inputs: &[Input],
        params: TxParameters,
    ) -> Self {
        let script_data: Vec<u8> = [to.to_vec(), amount.to_be_bytes().to_vec()]
            .into_iter()
            .flatten()
            .collect();

        // This script loads:
        //  - a pointer to the recipient address,
        //  - the amount
        // into the registers 0x10, 0x11
        // and calls the SMO instruction
        let script = vec![
            Opcode::gtf(0x10, 0x00, GTFArgs::ScriptData),
            Opcode::ADDI(0x11, 0x10, Bytes32::LEN as u16),
            Opcode::LW(0x11, 0x11, 0),
            Opcode::SMO(0x10, 0x00, 0x00, 0x11),
            Opcode::RET(REG_ONE),
        ]
        .into_iter()
        .collect();

        let outputs = vec![
            // when signing a transaction, recipient and amount are set to zero
            Output::message(Address::zeroed(), 0),
            Output::change(to, 0, BASE_ASSET_ID),
        ];

        let tx = Transaction::script(
            params.gas_price,
            params.gas_limit,
            params.maturity,
            script,
            script_data,
            inputs.to_vec(),
            outputs.to_vec(),
            vec![],
        );

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
