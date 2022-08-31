use anyhow::Result;
use fuel_gql_client::fuel_tx::{ConsensusParameters, Receipt, Transaction};
use fuel_gql_client::fuel_tx::{Input, Output, TxPointer, UtxoId};
use fuel_gql_client::fuel_types::{
    bytes::padded_len_usize, AssetId, Bytes32, ContractId, Immediate18, Word,
};
use fuel_gql_client::fuel_vm::{consts::REG_ONE, prelude::Opcode};
use itertools::{chain, Itertools};

use fuel_gql_client::client::schema::coin::Coin;
use fuels_core::parameters::TxParameters;
use fuels_signers::provider::Provider;
use fuels_signers::{Signer, WalletUnlocked};
use fuels_types::bech32::Bech32Address;
use fuels_types::{constants::WORD_SIZE, errors::Error};
use futures::{stream, StreamExt};
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
        calls: &[ContractCall],
        tx_parameters: &TxParameters,
        wallet: &WalletUnlocked,
    ) -> Result<Self, Error> {
        let data_offset = Self::get_data_offset(calls.len());

        let (script_data, call_param_offsets) = Self::get_script_data(calls, data_offset);

        let script = Self::get_instructions(calls, call_param_offsets);

        let required_asset_amounts = Self::calculate_required_asset_amounts(calls);
        let spendable_coins = Self::get_spendable_coins(wallet, &required_asset_amounts).await?;

        let (inputs, outputs) =
            Self::get_transaction_inputs_outputs(calls, wallet.address(), spendable_coins);

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

    /// Calculates how much of each asset id the given calls require and
    /// proceeds to request spendable coins from `wallet` to cover that cost.
    async fn get_spendable_coins(
        wallet: &WalletUnlocked,
        required_asset_amounts: &[(AssetId, u64)],
    ) -> Result<Vec<Coin>, Error> {
        stream::iter(required_asset_amounts)
            .map(|(asset_id, amount)| wallet.get_spendable_coins(*asset_id, *amount))
            .buffer_unordered(10)
            .collect::<Vec<_>>()
            .await
            .into_iter()
            .flatten_ok()
            .collect()
    }

    fn calculate_required_asset_amounts(calls: &[ContractCall]) -> Vec<(AssetId, u64)> {
        let amounts_per_asset_id = Self::extract_required_amounts_per_asset_id(calls);
        Self::sum_up_amounts_for_each_asset_id(amounts_per_asset_id)
    }

    fn extract_required_amounts_per_asset_id(
        calls: &[ContractCall],
    ) -> impl Iterator<Item = (AssetId, u64)> + '_ {
        calls
            .iter()
            .map(|call| (call.call_parameters.asset_id, call.call_parameters.amount))
    }

    fn sum_up_amounts_for_each_asset_id(
        amounts_per_asset_id: impl Iterator<Item = (AssetId, u64)>,
    ) -> Vec<(AssetId, u64)> {
        amounts_per_asset_id
            .group_by(|(asset_id, _)| *asset_id)
            .into_iter()
            .map(|(asset_id, groups_w_same_asset_id)| {
                let total_amount_in_group = groups_w_same_asset_id.map(|(_, amount)| amount).sum();
                (asset_id, total_amount_in_group)
            })
            .collect()
    }

    /// Given a list of contract calls, create the actual opcodes used to call the contract
    fn get_instructions(calls: &[ContractCall], offsets: Vec<CallParamOffsets>) -> Vec<u8> {
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
        calls: &[ContractCall],
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
    fn get_transaction_inputs_outputs(
        calls: &[ContractCall],
        wallet_address: &Bech32Address,
        spendable_coins: Vec<Coin>,
    ) -> (Vec<Input>, Vec<Output>) {
        let asset_ids = Self::extract_unique_asset_ids(&spendable_coins);
        let contract_ids = Self::extract_unique_contract_ids(calls);
        let num_of_contracts = contract_ids.len();

        let inputs = chain!(
            Self::generate_contract_inputs(contract_ids),
            Self::convert_to_signed_coins(spendable_coins),
        )
        .collect();

        // Note the contract_outputs need to come first since the
        // contract_inputs are referencing them via `output_index`. The node
        // will, upon receiving our request, use `output_index` to index the
        // `inputs` array we've sent over.
        let outputs = chain!(
            Self::generate_contract_outputs(num_of_contracts),
            Self::generate_asset_change_outputs(wallet_address, asset_ids),
            Self::extract_variable_outputs(calls),
        )
        .collect();

        (inputs, outputs)
    }

    fn extract_unique_asset_ids(spendable_coins: &[Coin]) -> HashSet<AssetId> {
        spendable_coins
            .iter()
            .map(|coin| coin.asset_id.clone().into())
            .collect()
    }

    fn extract_variable_outputs(calls: &[ContractCall]) -> Vec<Output> {
        calls
            .iter()
            .filter_map(|call| call.variable_outputs.clone())
            .flatten()
            .collect()
    }

    fn generate_asset_change_outputs(
        wallet_address: &Bech32Address,
        asset_ids: HashSet<AssetId>,
    ) -> Vec<Output> {
        asset_ids
            .into_iter()
            .map(|asset_id| Output::change(wallet_address.into(), 0, asset_id))
            .collect()
    }

    fn generate_contract_outputs(num_of_contracts: usize) -> Vec<Output> {
        (0..num_of_contracts)
            .map(|idx| Output::contract(idx as u8, Bytes32::zeroed(), Bytes32::zeroed()))
            .collect()
    }

    fn convert_to_signed_coins(spendable_coins: Vec<Coin>) -> Vec<Input> {
        spendable_coins
            .into_iter()
            .map(|coin| {
                Input::coin_signed(
                    UtxoId::from(coin.utxo_id),
                    coin.owner.into(),
                    coin.amount.0,
                    coin.asset_id.into(),
                    TxPointer::default(),
                    0,
                    0,
                )
            })
            .collect()
    }

    fn generate_contract_inputs(contract_ids: HashSet<ContractId>) -> Vec<Input> {
        contract_ids
            .into_iter()
            .enumerate()
            .map(|(idx, contract_id)| {
                Input::contract(
                    UtxoId::new(Bytes32::zeroed(), idx as u8),
                    Bytes32::zeroed(),
                    Bytes32::zeroed(),
                    TxPointer::default(),
                    contract_id,
                )
            })
            .collect()
    }

    fn extract_unique_contract_ids(calls: &[ContractCall]) -> HashSet<ContractId> {
        calls
            .iter()
            .flat_map(|call| {
                call.external_contracts
                    .iter()
                    .map(|bech32| bech32.into())
                    .chain(iter::once((&call.contract_id).into()))
            })
            .collect()
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
    use super::*;
    use fuel_gql_client::client::schema::coin::CoinStatus;
    use fuels_core::parameters::CallParameters;
    use fuels_types::bech32::Bech32ContractId;
    use rand::Rng;
    use std::slice;

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
        let (script_data, param_offsets) = Script::get_script_data(&calls, 0);

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

    #[test]
    fn contract_input_present() {
        let call = ContractCall::new_with_random_id();

        let (inputs, _) = Script::get_transaction_inputs_outputs(
            slice::from_ref(&call),
            &random_bech32_addr(),
            Default::default(),
        );

        assert_eq!(
            inputs,
            vec![Input::contract(
                UtxoId::new(Bytes32::zeroed(), 0),
                Bytes32::zeroed(),
                Bytes32::zeroed(),
                TxPointer::default(),
                call.contract_id.into(),
            )]
        );
    }

    #[test]
    fn contract_input_is_not_duplicated() {
        let call = ContractCall::new_with_random_id();
        let call_w_same_contract =
            ContractCall::new_with_random_id().with_contract_id(call.contract_id.clone());

        let calls = [call, call_w_same_contract];

        let (inputs, _) = Script::get_transaction_inputs_outputs(
            &calls,
            &random_bech32_addr(),
            Default::default(),
        );

        assert_eq!(
            inputs,
            vec![Input::contract(
                UtxoId::new(Bytes32::zeroed(), 0),
                Bytes32::zeroed(),
                Bytes32::zeroed(),
                TxPointer::default(),
                calls[0].contract_id.clone().into(),
            )]
        );
    }

    #[test]
    fn contract_output_present() {
        let call = ContractCall::new_with_random_id();

        let (_, outputs) = Script::get_transaction_inputs_outputs(
            &[call],
            &random_bech32_addr(),
            Default::default(),
        );

        assert_eq!(
            outputs,
            vec![Output::contract(0, Bytes32::zeroed(), Bytes32::zeroed())]
        );
    }

    #[test]
    fn external_contract_input_present() {
        // given
        let external_contract_id = random_bech32_contract_id();
        let call = ContractCall::new_with_random_id()
            .with_external_contracts(vec![external_contract_id.clone()]);

        // when
        let (inputs, _) = Script::get_transaction_inputs_outputs(
            slice::from_ref(&call),
            &random_bech32_addr(),
            Default::default(),
        );

        // then
        let mut expected_contract_ids: HashSet<ContractId> =
            [call.contract_id.into(), external_contract_id.into()].into();

        for (index, input) in inputs.into_iter().enumerate() {
            match input {
                Input::Contract {
                    utxo_id,
                    balance_root,
                    state_root,
                    tx_pointer,
                    contract_id,
                } => {
                    assert_eq!(utxo_id, UtxoId::new(Bytes32::zeroed(), index as u8));
                    assert_eq!(balance_root, Bytes32::zeroed());
                    assert_eq!(state_root, Bytes32::zeroed());
                    assert_eq!(tx_pointer, TxPointer::default());
                    assert!(expected_contract_ids.contains(&contract_id));
                    expected_contract_ids.remove(&contract_id);
                }
                _ => {
                    panic!("Expected only inputs of type Input::Contract");
                }
            }
        }
    }

    #[test]
    fn external_contract_output_present() {
        // given
        let external_contract_id = random_bech32_contract_id();
        let call =
            ContractCall::new_with_random_id().with_external_contracts(vec![external_contract_id]);

        // when
        let (_, outputs) = Script::get_transaction_inputs_outputs(
            &[call],
            &random_bech32_addr(),
            Default::default(),
        );

        // then
        let expected_outputs = (0..=1)
            .map(|i| Output::contract(i, Bytes32::zeroed(), Bytes32::zeroed()))
            .collect::<Vec<_>>();

        assert_eq!(outputs, expected_outputs);
    }

    #[test]
    fn change_per_asset_id_added() {
        // given
        let wallet_addr = random_bech32_addr();
        let asset_ids = [AssetId::default(), AssetId::from([1; 32])];

        let coins = asset_ids
            .into_iter()
            .map(|asset_id| Coin {
                amount: 100u64.into(),
                block_created: 0u64.into(),
                asset_id: asset_id.into(),
                utxo_id: Default::default(),
                maturity: 0u64.into(),
                owner: Default::default(),
                status: CoinStatus::Unspent,
            })
            .collect();
        let call = ContractCall::new_with_random_id();

        // when
        let (_, outputs) = Script::get_transaction_inputs_outputs(&[call], &wallet_addr, coins);

        // then
        let change_outputs: HashSet<Output> = outputs[1..].iter().cloned().collect();

        let expected_change_outputs = asset_ids
            .into_iter()
            .map(|asset_id| Output::Change {
                to: wallet_addr.clone().into(),
                amount: 0,
                asset_id,
            })
            .collect();

        assert_eq!(change_outputs, expected_change_outputs);
    }

    #[test]
    fn spendable_coins_added_to_input() {
        // given
        let asset_ids = [AssetId::default(), AssetId::from([1; 32])];

        let generate_spendable_coins = || {
            asset_ids
                .into_iter()
                .enumerate()
                .map(|(index, asset_id)| Coin {
                    amount: (index * 10).into(),
                    block_created: 1u64.into(),
                    asset_id: asset_id.into(),
                    utxo_id: Default::default(),
                    maturity: 0u64.into(),
                    owner: Default::default(),
                    status: CoinStatus::Unspent,
                })
                .collect::<Vec<_>>()
        };

        let call = ContractCall::new_with_random_id();

        // when
        let (inputs, _) = Script::get_transaction_inputs_outputs(
            &[call],
            &random_bech32_addr(),
            generate_spendable_coins(),
        );

        // then
        let inputs_as_signed_coins: HashSet<Input> = inputs[1..].iter().cloned().collect();

        let expected_inputs = generate_spendable_coins()
            .into_iter()
            .map(|coin| {
                Input::coin_signed(
                    fuel_tx::UtxoId::from(coin.utxo_id),
                    coin.owner.into(),
                    coin.amount.0,
                    coin.asset_id.into(),
                    TxPointer::default(),
                    0,
                    0,
                )
            })
            .collect::<HashSet<_>>();

        assert_eq!(expected_inputs, inputs_as_signed_coins);
    }

    #[test]
    fn variable_outputs_appended_to_outputs() {
        // given
        let variable_outputs = [100, 200].map(|amount| {
            Output::variable(random_bech32_addr().into(), amount, Default::default())
        });

        let calls = variable_outputs
            .iter()
            .cloned()
            .map(|variable_output| {
                ContractCall::new_with_random_id().with_variable_outputs(vec![variable_output])
            })
            .collect::<Vec<_>>();

        // when
        let (_, outputs) = Script::get_transaction_inputs_outputs(
            &calls,
            &random_bech32_addr(),
            Default::default(),
        );

        // then
        let actual_variable_outputs: HashSet<Output> = outputs[2..].iter().cloned().collect();
        let expected_outputs: HashSet<Output> = variable_outputs.into();

        assert_eq!(expected_outputs, actual_variable_outputs);
    }

    #[test]
    fn will_collate_same_asset_ids() {
        let amounts = [100, 200];

        let asset_id = [1; 32].into();
        let calls = amounts.map(|amount| {
            ContractCall::new_with_random_id().with_call_parameters(CallParameters {
                amount,
                asset_id,
                gas_forwarded: 0,
            })
        });

        let asset_id_amounts = Script::calculate_required_asset_amounts(&calls);

        let expected_asset_id_amounts = [(asset_id, amounts.iter().sum())].into();

        assert_eq!(
            asset_id_amounts.into_iter().collect::<HashSet<_>>(),
            expected_asset_id_amounts
        )
    }

    impl ContractCall {
        pub fn new_with_random_id() -> Self {
            ContractCall {
                contract_id: random_bech32_contract_id(),
                encoded_args: Default::default(),
                encoded_selector: [0; 8],
                call_parameters: Default::default(),
                compute_custom_input_offset: false,
                variable_outputs: None,
                external_contracts: Default::default(),
                output_param: None,
            }
        }
    }

    impl ContractCall {
        pub fn with_contract_id(self, contract_id: Bech32ContractId) -> Self {
            ContractCall {
                contract_id,
                ..self
            }
        }
        pub fn with_external_contracts(
            self,
            external_contracts: Vec<Bech32ContractId>,
        ) -> ContractCall {
            ContractCall {
                external_contracts,
                ..self
            }
        }

        pub fn with_variable_outputs(self, variable_outputs: Vec<Output>) -> ContractCall {
            ContractCall {
                variable_outputs: Some(variable_outputs),
                ..self
            }
        }

        pub fn with_call_parameters(self, call_parameters: CallParameters) -> ContractCall {
            ContractCall {
                call_parameters,
                ..self
            }
        }
    }

    fn random_bech32_addr() -> Bech32Address {
        Bech32Address::new("fuel", rand::thread_rng().gen::<[u8; 32]>())
    }

    fn random_bech32_contract_id() -> Bech32ContractId {
        Bech32ContractId::new("fuel", rand::thread_rng().gen::<[u8; 32]>())
    }
}
