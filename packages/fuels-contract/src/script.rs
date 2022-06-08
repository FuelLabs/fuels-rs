use anyhow::Result;
use fuel_gql_client::fuel_tx::{Input, Output, UtxoId};
use fuel_gql_client::fuel_types::{
    bytes::padded_len_usize, AssetId, Bytes32, ContractId, Immediate18, Word,
};
use fuel_gql_client::fuel_vm::consts::VM_TX_MEMORY;
use fuel_gql_client::fuel_vm::{
    consts::{REG_CGAS, REG_ONE},
    prelude::Opcode,
};
use fuel_gql_client::{
    client::{types::TransactionStatus, FuelClient},
    fuel_tx::{Receipt, Transaction},
};
use fuels_core::constants::{DEFAULT_SPENDABLE_COIN_AMOUNT, WORD_SIZE};
use fuels_core::errors::Error;
use fuels_core::parameters::TxParameters;

use crate::contract::ContractCall;
use fuels_signers::{LocalWallet, Signer};

/// Script provides methods to create and a call/simulate a
/// script transaction that carries out contract method calls
pub struct Script {
    pub tx: Transaction,
}

#[derive(Debug, Clone)]
pub struct CompiledScript {
    pub raw: Vec<u8>,
    pub target_network_url: String,
}

impl Script {
    pub fn new(tx: Transaction) -> Self {
        Self { tx }
    }

    /// Creates a Script from a contract call. The internal Transaction field is initialized
    /// with the actual script instructions and script data needed to perform the call,
    /// and transaction inputs/outputs consisting of assets, external contract ids etc.
    pub async fn from_contract_call(
        call: &ContractCall,
        tx_parameters: &TxParameters,
        wallet: &LocalWallet,
    ) -> Self {
        let (script, offset) = Self::get_instructions(vec![call]);

        let script_data = Self::get_script_data_from_calls(vec![call], offset);

        let mut inputs: Vec<Input> = vec![];
        let mut outputs: Vec<Output> = vec![];

        let self_contract_input = Input::contract(
            UtxoId::new(Bytes32::zeroed(), 0),
            Bytes32::zeroed(),
            Bytes32::zeroed(),
            call.contract_id,
        );
        inputs.push(self_contract_input);

        let mut spendables = wallet
            .get_spendable_coins(&AssetId::default(), DEFAULT_SPENDABLE_COIN_AMOUNT as u64)
            .await
            .unwrap();

        // add default asset change if any inputs are being spent
        if !spendables.is_empty() {
            let change_output = Output::change(wallet.address(), 0, AssetId::default());
            outputs.push(change_output);
        }

        if call.call_parameters.asset_id != AssetId::default() {
            let alt_spendables = wallet
                .get_spendable_coins(&call.call_parameters.asset_id, call.call_parameters.amount)
                .await
                .unwrap();

            // add alt change if inputs are being spent
            if !alt_spendables.is_empty() {
                let change_output =
                    Output::change(wallet.address(), 0, call.call_parameters.asset_id);
                outputs.push(change_output);
            }

            // add alt coins to inputs
            spendables.extend(alt_spendables.into_iter());
        }

        for coin in spendables {
            let input_coin = Input::coin_signed(
                UtxoId::from(coin.utxo_id),
                coin.owner.into(),
                coin.amount.0,
                coin.asset_id.into(),
                0,
                0,
            );

            inputs.push(input_coin);
        }

        let n_inputs = inputs.len();

        let self_contract_output = Output::contract(0, Bytes32::zeroed(), Bytes32::zeroed());
        outputs.push(self_contract_output);

        // Add external contract IDs to Input/Output pair, if applicable.
        if let Some(external_contract_ids) = call.external_contracts.clone() {
            for (idx, external_contract_id) in external_contract_ids.iter().enumerate() {
                // We must associate the right external contract input to the corresponding external
                // output index (TXO). We add the `n_inputs` offset because we added some inputs
                // above.
                let output_index: u8 = (idx + n_inputs) as u8;
                let zeroes = Bytes32::zeroed();
                let external_contract_input = Input::contract(
                    UtxoId::new(Bytes32::zeroed(), output_index),
                    zeroes,
                    zeroes,
                    *external_contract_id,
                );

                inputs.push(external_contract_input);

                let external_contract_output = Output::contract(output_index, zeroes, zeroes);

                outputs.push(external_contract_output);
            }
        }

        // Add outputs to the transaction.
        if let Some(v) = call.variable_outputs.clone() {
            outputs.extend(v);
        };

        let mut tx = Transaction::script(
            tx_parameters.gas_price,
            tx_parameters.gas_limit,
            tx_parameters.byte_price,
            call.maturity,
            script,
            script_data,
            inputs,
            outputs,
            vec![],
        );
        wallet.sign_transaction(&mut tx).await.unwrap();

        Script::new(tx)
    }

    /// Given a list of contract calls, create the actual opcodes used to call the contract
    fn get_instructions(calls: Vec<&ContractCall>) -> (Vec<u8>, usize) {
        let num_calls = calls.len();
        let offset = Self::get_data_offset(num_calls);

        let mut instructions = vec![];
        for _ in 0..num_calls {
            instructions.extend(Self::single_call_instructions(offset, 0));
        }

        instructions.extend(Opcode::RET(REG_ONE).to_bytes());

        (instructions, offset)
    }

    /// Returns script data, consisting of the following items in the given order:
    /// 1. Asset ID to be forwarded (AmountId::LEN)
    /// 2. Amount to be forwarded (1 * WORD_SIZE)
    /// 3. Contract ID (ContractID::LEN);
    /// 4. Function selector (1 * WORD_SIZE);
    /// 5. Calldata offset (optional) (1 * WORD_SIZE)
    /// 6. Encoded arguments (optional) (variable length)
    fn get_script_data_from_calls(calls: Vec<&ContractCall>, offset: usize) -> Vec<u8> {
        let mut script_data: Vec<u8> = vec![];

        for call in calls {
            script_data.extend(call.call_parameters.asset_id.to_vec());

            let amount = call.call_parameters.amount as Word;
            script_data.extend(amount.to_be_bytes());

            script_data.extend(call.contract_id.as_ref());

            script_data.extend(call.encoded_selector);

            // If the method call takes custom inputs or has more than
            // one argument, we need to calculate the `call_data_offset`,
            // which points to where the data for the custom types start in the
            // transaction. If it doesn't take any custom inputs, this isn't necessary.
            if call.compute_calldata_offset {
                // Offset of the script data relative to the call data
                let call_data_offset =
                    offset + AssetId::LEN + WORD_SIZE + ContractId::LEN + 2 * WORD_SIZE;
                let call_data_offset = call_data_offset as Word;

                script_data.extend(&call_data_offset.to_be_bytes());
            }
            script_data.extend(call.encoded_args.clone());
        }

        script_data
    }

    /// Returns the VM instructions for calling a contract method
    /// We use the Opcode to call a contract: `CALL` pointing at the
    /// following registers;
    ///
    /// 0x10 Script data offset
    /// 0x11 Gas price TODO: #184
    /// 0x12 Coin amount
    /// 0x13 Asset ID
    ///
    /// Note that these are soft rules as we're picking this addresses simply because they
    /// non-reserved register.
    fn single_call_instructions(data_offset: usize, segment_offset: usize) -> Vec<u8> {
        let instructions = vec![
            Opcode::MOVI(
                0x10,
                (data_offset + segment_offset + AssetId::LEN + WORD_SIZE) as Immediate18,
            ),
            Opcode::MOVI(
                0x12,
                (data_offset + segment_offset + AssetId::LEN) as Immediate18,
            ),
            Opcode::LW(0x12, 0x12, 0),
            Opcode::MOVI(0x13, (data_offset + segment_offset) as Immediate18),
            Opcode::CALL(0x10, 0x12, 0x13, REG_CGAS),
        ];

        #[allow(clippy::iter_cloned_collect)]
        instructions.iter().copied().collect::<Vec<u8>>()
    }

    /// Returns the offset between the script instructions and the script data in memory
    /// based on the amount of contract calls the script has to make
    fn get_data_offset(num_calls: usize) -> usize {
        let mut len_script = Script::single_call_instructions(0, 0).len() * num_calls;

        // to account for RET instruction which is added later
        len_script += Opcode::LEN;

        VM_TX_MEMORY + Transaction::script_offset() + padded_len_usize(len_script)
    }

    /// Execute the transaction in a state-modifying manner.
    pub async fn call(self, fuel_client: &FuelClient) -> Result<Vec<Receipt>, Error> {
        let tx_id = fuel_client.submit(&self.tx).await?.0.to_string();
        let receipts = fuel_client.receipts(&tx_id).await?;
        let status = fuel_client.transaction_status(&tx_id).await?;
        match status {
            TransactionStatus::Failure { reason, .. } => {
                Err(Error::ContractCallError(reason, receipts))
            }
            _ => Ok(receipts),
        }
    }

    /// Execute the transaction in a simulated manner, not modifying blockchain state
    pub async fn simulate(self, fuel_client: &FuelClient) -> Result<Vec<Receipt>, Error> {
        let receipts = fuel_client.dry_run(&self.tx).await?;
        Ok(receipts)
    }
}
