use anyhow::Result;
use fuel_gql_client::fuel_tx::{ConsensusParameters, Receipt, Transaction};
use fuel_gql_client::fuel_tx::{Input, Output, UtxoId};
use fuel_gql_client::fuel_types::{
    bytes::padded_len_usize, AssetId, Bytes32, ContractId, Immediate18, Word,
};
use fuel_gql_client::fuel_vm::{consts::REG_ONE, prelude::Opcode};

use fuels_core::constants::DEFAULT_SPENDABLE_COIN_AMOUNT;
use fuels_core::parameters::TxParameters;
use fuels_signers::provider::Provider;
use fuels_signers::{LocalWallet, Signer};
use fuels_types::{constants::WORD_SIZE, errors::Error};
use std::collections::HashSet;
use std::iter;

use crate::contract::ContractCall;

#[derive(Default)]
/// Specifies offsets of Opcode::CALL parameters stored in the script
/// data from which they can be loaded into registers
struct CallParamOffsets {
    pub asset_id_offset: usize,
    pub amount_offset: usize,
    pub gas_forwarded_offset: usize,
    pub call_data_offset: usize,
}

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

    /// Creates a Script from a contract call. The internal Transaction is initialized
    /// with the actual script instructions, script data needed to perform the call
    /// and transaction inputs/outputs consisting of assets and contracts
    pub async fn from_contract_calls(
        calls: Vec<&ContractCall>,
        tx_parameters: &TxParameters,
        wallet: &LocalWallet,
    ) -> Self {
        let data_offset = Self::get_data_offset(calls.len());

        let (script_data, call_param_offsets) = Self::get_script_data(calls.clone(), data_offset);

        let script = Self::get_instructions(calls.clone(), call_param_offsets);

        let (inputs, outputs) = Self::get_transaction_inputs_outputs(calls.clone(), wallet).await;

        let mut tx = Transaction::script(
            tx_parameters.gas_price,
            tx_parameters.gas_limit,
            tx_parameters.byte_price,
            tx_parameters.maturity,
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
    fn get_instructions(calls: Vec<&ContractCall>, offsets: Vec<CallParamOffsets>) -> Vec<u8> {
        let num_calls = calls.len();

        let mut instructions = vec![];
        for (_, call_offsets) in (0..num_calls).zip(offsets.iter()) {
            instructions.extend(Self::get_single_call_instructions(call_offsets));
        }

        instructions.extend(Opcode::RET(REG_ONE).to_bytes());

        instructions
    }

    /// Returns script data, consisting of the following items in the given order:
    /// 1. Asset ID to be forwarded (AmountId::LEN)
    /// 2. Amount to be forwarded (1 * WORD_SIZE)
    /// 3. Gas to be forwarded (1 * WORD_SIZE)
    /// 4. Contract ID (ContractID::LEN);
    /// 5. Function selector (1 * WORD_SIZE);
    /// 6. Calldata offset (optional) (1 * WORD_SIZE)
    /// 7. Encoded arguments (optional) (variable length)
    fn get_script_data(
        calls: Vec<&ContractCall>,
        data_offset: usize,
    ) -> (Vec<u8>, Vec<CallParamOffsets>) {
        let mut script_data = vec![];
        let mut param_offsets = vec![];

        // The data for each call is ordered into segments
        let mut segment_offset = data_offset;

        for call in calls {
            let call_param_offsets = CallParamOffsets {
                asset_id_offset: segment_offset,
                amount_offset: segment_offset + AssetId::LEN,
                gas_forwarded_offset: segment_offset + AssetId::LEN + WORD_SIZE,
                call_data_offset: segment_offset + AssetId::LEN + 2 * WORD_SIZE,
            };
            param_offsets.push(call_param_offsets);

            script_data.extend(call.call_parameters.asset_id.to_vec());

            script_data.extend(call.call_parameters.amount.to_be_bytes());

            script_data.extend(call.call_parameters.gas_forwarded.to_be_bytes());

            script_data.extend(call.contract_id.hash().as_ref());

            script_data.extend(call.encoded_selector);

            // If the method call takes custom inputs or has more than
            // one argument, we need to calculate the `call_data_offset`,
            // which points to where the data for the custom types start in the
            // transaction. If it doesn't take any custom inputs, this isn't necessary.
            if call.compute_custom_input_offset {
                // Custom inputs are stored after the previously added parameters,
                // including custom_input_offset
                let custom_input_offset =
                    segment_offset + AssetId::LEN + 2 * WORD_SIZE + ContractId::LEN + 2 * WORD_SIZE;
                let custom_input_offset = custom_input_offset as Word;

                script_data.extend(&custom_input_offset.to_be_bytes());
            }

            script_data.extend(call.encoded_args.clone());

            // the data segment that holds the parameters for the next call
            // begins at the original offset + the data we added so far
            segment_offset = data_offset + script_data.len();
        }

        (script_data, param_offsets)
    }

    /// Returns the VM instructions for calling a contract method
    /// We use the Opcode to call a contract: `CALL` pointing at the
    /// following registers;
    ///
    /// 0x10 Script data offset
    /// 0x11 Gas forwarded
    /// 0x12 Coin amount
    /// 0x13 Asset ID
    ///
    /// Note that these are soft rules as we're picking this addresses simply because they
    /// non-reserved register.
    fn get_single_call_instructions(offsets: &CallParamOffsets) -> Vec<u8> {
        let instructions = vec![
            Opcode::MOVI(0x10, offsets.call_data_offset as Immediate18),
            Opcode::MOVI(0x11, offsets.gas_forwarded_offset as Immediate18),
            Opcode::LW(0x11, 0x11, 0),
            Opcode::MOVI(0x12, offsets.amount_offset as Immediate18),
            Opcode::LW(0x12, 0x12, 0),
            Opcode::MOVI(0x13, offsets.asset_id_offset as Immediate18),
            Opcode::CALL(0x10, 0x12, 0x13, 0x11),
        ];

        #[allow(clippy::iter_cloned_collect)]
        instructions.iter().copied().collect::<Vec<u8>>()
    }

    /// Returns the assets and contracts that will be consumed (inputs) and created (outputs)
    /// by the transaction
    async fn get_transaction_inputs_outputs(
        calls: Vec<&ContractCall>,
        wallet: &LocalWallet,
    ) -> (Vec<Input>, Vec<Output>) {
        let mut inputs: Vec<Input> = vec![];
        let mut outputs: Vec<Output> = vec![];

        // Get all unique contract ids
        let contract_ids: HashSet<ContractId> = calls
            .iter()
            .flat_map(|call| {
                let mut ids: HashSet<ContractId> = call
                    .external_contracts
                    .iter()
                    .map(|bech32| ContractId::new(*bech32.hash()))
                    .collect();
                ids.insert((&call.contract_id).into());
                ids
            })
            .collect();

        // We must associate the right external contract input to the corresponding external
        // output index (TXO)
        for (idx, contract_id) in contract_ids.into_iter().enumerate() {
            let zeroes = Bytes32::zeroed();
            let self_contract_input = Input::contract(
                UtxoId::new(Bytes32::zeroed(), idx as u8),
                zeroes,
                zeroes,
                contract_id,
            );
            inputs.push(self_contract_input);

            let external_contract_output = Output::contract(idx as u8, zeroes, zeroes);
            outputs.push(external_contract_output);
        }

        // Get all unique asset ids
        let asset_ids: HashSet<AssetId> = calls
            .iter()
            .map(|call| call.call_parameters.asset_id)
            .chain(iter::once(AssetId::default()))
            .collect();

        let mut spendables = vec![];
        for asset_id in asset_ids.iter() {
            spendables.extend(
                wallet
                    .get_spendable_coins(asset_id, DEFAULT_SPENDABLE_COIN_AMOUNT as u64)
                    .await
                    .unwrap(),
            );
        }

        for asset_id in asset_ids.iter() {
            // add asset change if any inputs are being spent
            let change_output = Output::change(wallet.address().into(), 0, asset_id.to_owned());
            outputs.push(change_output);
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

        calls.iter().for_each(|call| {
            if let Some(v) = call.variable_outputs.clone() {
                outputs.extend(v);
            };
        });

        (inputs, outputs)
    }

    /// Calculates the length of the script based on the number of contract calls it
    /// has to make and returns the offset at which the script data begins
    fn get_data_offset(num_calls: usize) -> usize {
        // use placeholder for call param offsets, we only care about the length
        let mut len_script =
            Script::get_single_call_instructions(&CallParamOffsets::default()).len() * num_calls;

        // to account for RET instruction which is added later
        len_script += Opcode::LEN;

        ConsensusParameters::DEFAULT.tx_offset()
            + Transaction::script_offset()
            + padded_len_usize(len_script)
    }

    /// Execute the transaction in a state-modifying manner.
    pub async fn call(self, provider: &Provider) -> Result<Vec<Receipt>, Error> {
        let chain_info = provider.chain_info().await?;

        self.tx.validate_without_signature(
            chain_info.latest_block.height.0,
            &chain_info.consensus_parameters.into(),
        )?;

        provider.send_transaction(&self.tx).await
    }

    /// Execute the transaction in a simulated manner, not modifying blockchain state
    pub async fn simulate(self, provider: &Provider) -> Result<Vec<Receipt>, Error> {
        let chain_info = provider.chain_info().await?;

        self.tx.validate_without_signature(
            chain_info.latest_block.height.0,
            &chain_info.consensus_parameters.into(),
        )?;

        let receipts = provider.dry_run(&self.tx).await?;
        Ok(receipts)
    }
}

#[cfg(test)]
mod test {
    use fuels_core::parameters::CallParameters;
    use fuels_types::bech32::Bech32ContractId;

    use super::*;

    #[tokio::test]
    async fn test_script_data() {
        // Arrange
        const SELECTOR_LEN: usize = WORD_SIZE;
        const NUM_CALLS: usize = 3;

        let contract_ids = vec![
            Bech32ContractId::new("test", Bytes32::new([1u8; 32])),
            Bech32ContractId::new("test", Bytes32::new([1u8; 32])),
            Bech32ContractId::new("test", Bytes32::new([1u8; 32])),
        ];

        let asset_ids = vec![
            AssetId::from([4u8; 32]),
            AssetId::from([5u8; 32]),
            AssetId::from([6u8; 32]),
        ];

        let selectors = vec![[7u8; 8], [8u8; 8], [9u8; 8]];

        // Call 2 has a multiple inputs, compute_custom_input_offset will be true
        let args = vec![[10u8; 8].to_vec(), [11u8; 16].to_vec(), [12u8; 8].to_vec()];

        let calls: Vec<ContractCall> = (0..NUM_CALLS)
            .map(|i| ContractCall {
                contract_id: contract_ids[i].clone(),
                encoded_selector: selectors[i],
                encoded_args: args[i].clone(),
                call_parameters: CallParameters::new(
                    Some(i as u64),
                    Some(asset_ids[i]),
                    Some(i as u64),
                ),
                compute_custom_input_offset: i == 1,
                variable_outputs: None,
                external_contracts: vec![],
                output_param: None,
            })
            .collect();

        // Act
        let (script_data, param_offsets) = Script::get_script_data(calls.iter().collect(), 0);

        // Assert
        assert_eq!(param_offsets.len(), NUM_CALLS);
        for (idx, offsets) in param_offsets.iter().enumerate() {
            let asset_id = script_data
                [offsets.asset_id_offset..offsets.asset_id_offset + AssetId::LEN]
                .to_vec();
            assert_eq!(asset_id, asset_ids[idx].to_vec());

            let amount =
                script_data[offsets.amount_offset..offsets.amount_offset + WORD_SIZE].to_vec();
            assert_eq!(amount, idx.to_be_bytes());

            let gas = script_data
                [offsets.gas_forwarded_offset..offsets.gas_forwarded_offset + WORD_SIZE]
                .to_vec();
            assert_eq!(gas, idx.to_be_bytes().to_vec());

            let contract_id =
                &script_data[offsets.call_data_offset..offsets.call_data_offset + ContractId::LEN];
            let expected_contract_id = contract_ids[idx].hash();
            assert_eq!(contract_id, expected_contract_id.as_slice());

            let selector_offset = offsets.call_data_offset + ContractId::LEN;
            let selector = script_data[selector_offset..selector_offset + SELECTOR_LEN].to_vec();
            assert_eq!(selector, selectors[idx].to_vec());
        }

        // Calls 1 and 3 have their input arguments after the selector
        let call_1_arg_offset = param_offsets[0].call_data_offset + ContractId::LEN + SELECTOR_LEN;
        let call_1_arg = script_data[call_1_arg_offset..call_1_arg_offset + WORD_SIZE].to_vec();
        assert_eq!(call_1_arg, args[0].to_vec());

        let call_3_arg_offset = param_offsets[2].call_data_offset + ContractId::LEN + SELECTOR_LEN;
        let call_3_arg = script_data[call_3_arg_offset..call_3_arg_offset + WORD_SIZE].to_vec();
        assert_eq!(call_3_arg, args[2]);

        // Call 2 has custom inputs and custom_input_offset
        let call_2_arg_offset = param_offsets[1].call_data_offset + ContractId::LEN + SELECTOR_LEN;
        let custom_input_offset =
            script_data[call_2_arg_offset..call_2_arg_offset + WORD_SIZE].to_vec();
        assert_eq!(
            custom_input_offset,
            (call_2_arg_offset + WORD_SIZE).to_be_bytes()
        );

        let custom_input_offset =
            param_offsets[1].call_data_offset + ContractId::LEN + SELECTOR_LEN + WORD_SIZE;
        let custom_input =
            script_data[custom_input_offset..custom_input_offset + 2 * WORD_SIZE].to_vec();
        assert_eq!(custom_input, args[1]);
    }
}
