use std::{collections::HashSet, iter, vec};

use fuel_tx::{
    AssetId, Bytes32, ContractId, Input, Output, Receipt, ScriptExecutionResult, TxPointer, UtxoId,
};
use fuel_types::Word;
use fuel_vm::fuel_asm::{op, RegId};
use fuels_core::offsets::call_script_data_offset;
use fuels_signers::{provider::Provider, Signer, WalletUnlocked};
use fuels_types::{
    bech32::Bech32Address,
    constants::{BASE_ASSET_ID, WORD_SIZE},
    errors::{Error, Result},
    parameters::TxParameters,
    resource::Resource,
    transaction::{ScriptTransaction, Transaction},
};
use itertools::{chain, Itertools};

use crate::contract::ContractCall;

#[derive(Default)]
/// Specifies offsets of [`Instruction::CALL`] parameters stored in the script
/// data from which they can be loaded into registers
pub(crate) struct CallOpcodeParamsOffset {
    pub asset_id_offset: usize,
    pub amount_offset: usize,
    pub gas_forwarded_offset: usize,
    pub call_data_offset: usize,
}

/// Creates a [`ScriptTransaction`] from contract calls. The internal [Transaction] is
/// initialized with the actual script instructions, script data needed to perform the call and
/// transaction inputs/outputs consisting of assets and contracts.
pub async fn build_tx_from_contract_calls(
    calls: &[ContractCall],
    tx_parameters: &TxParameters,
    wallet: &WalletUnlocked,
) -> Result<ScriptTransaction> {
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

    let mut tx = ScriptTransaction::new(inputs, outputs, *tx_parameters)
        .with_script(script)
        .with_script_data(script_data);

    let base_asset_amount = required_asset_amounts
        .iter()
        .find(|(asset_id, _)| *asset_id == AssetId::default());
    match base_asset_amount {
        Some((_, base_amount)) => wallet.add_fee_resources(&mut tx, *base_amount, 0).await?,
        None => wallet.add_fee_resources(&mut tx, 0, 0).await?,
    }
    wallet.sign_transaction(&mut tx).await.unwrap();

    Ok(tx)
}

/// Compute how much of each asset is required based on all `CallParameters` of the `ContractCalls`
pub(crate) fn calculate_required_asset_amounts(calls: &[ContractCall]) -> Vec<(AssetId, u64)> {
    let call_param_assets = calls
        .iter()
        .map(|call| (call.call_parameters.asset_id, call.call_parameters.amount))
        .collect::<Vec<_>>();

    let custom_assets = calls
        .iter()
        .flat_map(|call| call.custom_assets.iter().collect::<Vec<_>>())
        .group_by(|custom| custom.0 .0)
        .into_iter()
        .map(|(asset_id, groups_w_same_asset_id)| {
            let total_amount_in_group = groups_w_same_asset_id.map(|(_, amount)| amount).sum();
            (asset_id, total_amount_in_group)
        })
        .collect::<Vec<_>>();

    let merged_assets = chain!(call_param_assets, custom_assets).collect::<Vec<_>>();

    sum_up_amounts_for_each_asset_id(merged_assets)
}

/// Sum up the amounts required in each call for each asset ID, so you can get a total for each
/// asset over all calls.
fn sum_up_amounts_for_each_asset_id(
    amounts_per_asset_id: Vec<(AssetId, u64)>,
) -> Vec<(AssetId, u64)> {
    amounts_per_asset_id
        .into_iter()
        .sorted_by_key(|(asset_id, _)| *asset_id)
        .group_by(|(asset_id, _)| *asset_id)
        .into_iter()
        .map(|(asset_id, groups_w_same_asset_id)| {
            let total_amount_in_group = groups_w_same_asset_id.map(|(_, amount)| amount).sum();
            (asset_id, total_amount_in_group)
        })
        .collect()
}

/// Given a list of contract calls, create the actual opcodes used to call the contract
pub(crate) fn get_instructions(
    calls: &[ContractCall],
    offsets: Vec<CallOpcodeParamsOffset>,
) -> Vec<u8> {
    let num_calls = calls.len();

    let mut instructions = vec![];
    for (_, call_offsets) in (0..num_calls).zip(offsets.iter()) {
        instructions.extend(get_single_call_instructions(call_offsets));
    }

    instructions.extend(op::ret(RegId::ONE).to_bytes());

    instructions
}

/// Returns script data, consisting of the following items in the given order:
/// 1. Asset ID to be forwarded ([`AssetId::LEN`])
/// 2. Amount to be forwarded `(1 * `[`WORD_SIZE`]`)`
/// 3. Gas to be forwarded `(1 * `[`WORD_SIZE`]`)`
/// 4. Contract ID ([`ContractId::LEN`]);
/// 5. Function selector `(1 * `[`WORD_SIZE`]`)`
/// 6. Calldata offset (optional) `(1 * `[`WORD_SIZE`]`)`
/// 7. Encoded arguments (optional) (variable length)
pub(crate) fn build_script_data_from_contract_calls(
    calls: &[ContractCall],
    data_offset: usize,
    gas_limit: u64,
) -> (Vec<u8>, Vec<CallOpcodeParamsOffset>) {
    let mut script_data = vec![];
    let mut param_offsets = vec![];

    // The data for each call is ordered into segments
    let mut segment_offset = data_offset;

    for call in calls {
        let call_param_offsets = CallOpcodeParamsOffset {
            asset_id_offset: segment_offset,
            amount_offset: segment_offset + AssetId::LEN,
            gas_forwarded_offset: segment_offset + AssetId::LEN + WORD_SIZE,
            call_data_offset: segment_offset + AssetId::LEN + 2 * WORD_SIZE,
        };
        param_offsets.push(call_param_offsets);

        script_data.extend(call.call_parameters.asset_id.to_vec());

        script_data.extend(call.call_parameters.amount.to_be_bytes());

        // If gas_forwarded is not set, use the transaction gas limit
        let gas_forwarded = call.call_parameters.gas_forwarded.unwrap_or(gas_limit);
        script_data.extend(gas_forwarded.to_be_bytes());

        script_data.extend(call.contract_id.hash().as_ref());

        script_data.extend(call.encoded_selector);

        // If the method call takes custom inputs or has more than
        // one argument, we need to calculate the `call_data_offset`,
        // which points to where the data for the custom types start in the
        // transaction. If it doesn't take any custom inputs, this isn't necessary.
        let encoded_args_start_offset = if call.compute_custom_input_offset {
            // Custom inputs are stored after the previously added parameters,
            // including custom_input_offset
            let custom_input_offset =
                segment_offset + AssetId::LEN + 2 * WORD_SIZE + ContractId::LEN + 2 * WORD_SIZE;
            script_data.extend((custom_input_offset as Word).to_be_bytes());
            custom_input_offset
        } else {
            segment_offset
        };

        let bytes = call.encoded_args.resolve(encoded_args_start_offset as u64);
        script_data.extend(bytes);

        // the data segment that holds the parameters for the next call
        // begins at the original offset + the data we added so far
        segment_offset = data_offset + script_data.len();
    }

    (script_data, param_offsets)
}

/// Returns the VM instructions for calling a contract method
/// We use the [`Opcode`] to call a contract: [`CALL`](Opcode::CALL)
/// pointing at the following registers:
///
/// 0x10 Script data offset
/// 0x11 Gas forwarded
/// 0x12 Coin amount
/// 0x13 Asset ID
///
/// Note that these are soft rules as we're picking this addresses simply because they
/// non-reserved register.
pub(crate) fn get_single_call_instructions(offsets: &CallOpcodeParamsOffset) -> Vec<u8> {
    let call_data_offset = offsets
        .call_data_offset
        .try_into()
        .expect("call_data_offset out of range");
    let gas_forwarded_offset = offsets
        .gas_forwarded_offset
        .try_into()
        .expect("gas_forwarded_offset out of range");
    let amount_offset = offsets
        .amount_offset
        .try_into()
        .expect("amount_offset out of range");
    let asset_id_offset = offsets
        .asset_id_offset
        .try_into()
        .expect("asset_id_offset out of range");
    let instructions = [
        op::movi(0x10, call_data_offset),
        op::movi(0x11, gas_forwarded_offset),
        op::lw(0x11, 0x11, 0),
        op::movi(0x12, amount_offset),
        op::lw(0x12, 0x12, 0),
        op::movi(0x13, asset_id_offset),
        op::call(0x10, 0x12, 0x13, 0x11),
    ];

    #[allow(clippy::iter_cloned_collect)]
    instructions.into_iter().collect::<Vec<u8>>()
}

/// Returns the assets and contracts that will be consumed ([`Input`]s)
/// and created ([`Output`]s) by the transaction
pub(crate) fn get_transaction_inputs_outputs(
    calls: &[ContractCall],
    wallet_address: &Bech32Address,
    spendable_resources: Vec<Resource>,
) -> (Vec<Input>, Vec<Output>) {
    let asset_ids = extract_unique_asset_ids(&spendable_resources);
    let contract_ids = extract_unique_contract_ids(calls);
    let num_of_contracts = contract_ids.len();

    let inputs = chain!(
        generate_contract_inputs(contract_ids),
        convert_to_signed_resources(spendable_resources),
    )
    .collect();

    // Note the contract_outputs need to come first since the
    // contract_inputs are referencing them via `output_index`. The node
    // will, upon receiving our request, use `output_index` to index the
    // `inputs` array we've sent over.
    let outputs = chain!(
        generate_contract_outputs(num_of_contracts),
        generate_asset_change_outputs(wallet_address, asset_ids),
        generate_custom_outputs(calls),
        extract_variable_outputs(calls),
        extract_message_outputs(calls)
    )
    .collect();
    (inputs, outputs)
}

fn generate_custom_outputs(calls: &[ContractCall]) -> Vec<Output> {
    calls
        .iter()
        .flat_map(|call| &call.custom_assets)
        .group_by(|custom| (custom.0 .0, custom.0 .1.clone()))
        .into_iter()
        .filter_map(|(asset_id_address, groups_w_same_asset_id_address)| {
            let total_amount_in_group = groups_w_same_asset_id_address
                .map(|(_, amount)| amount)
                .sum::<u64>();
            match asset_id_address.1 {
                Some(address) => Some(Output::coin(
                    address.into(),
                    total_amount_in_group,
                    asset_id_address.0,
                )),
                None => None,
            }
        })
        .collect::<Vec<_>>()
}

fn extract_unique_asset_ids(spendable_coins: &[Resource]) -> HashSet<AssetId> {
    spendable_coins
        .iter()
        .map(|resource| match resource {
            Resource::Coin(coin) => coin.asset_id,
            Resource::Message(_) => BASE_ASSET_ID,
        })
        .collect()
}

fn extract_variable_outputs(calls: &[ContractCall]) -> Vec<Output> {
    calls
        .iter()
        .filter_map(|call| call.variable_outputs.clone())
        .flatten()
        .collect()
}

fn extract_message_outputs(calls: &[ContractCall]) -> Vec<Output> {
    calls
        .iter()
        .filter_map(|call| call.message_outputs.clone())
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

pub(crate) fn generate_contract_outputs(num_of_contracts: usize) -> Vec<Output> {
    (0..num_of_contracts)
        .map(|idx| Output::contract(idx as u8, Bytes32::zeroed(), Bytes32::zeroed()))
        .collect()
}

fn convert_to_signed_resources(spendable_resources: Vec<Resource>) -> Vec<Input> {
    spendable_resources
        .into_iter()
        .map(|resource| match resource {
            Resource::Coin(coin) => Input::coin_signed(
                coin.utxo_id,
                coin.owner.into(),
                coin.amount,
                coin.asset_id,
                TxPointer::default(),
                0,
                coin.maturity,
            ),
            Resource::Message(message) => Input::message_signed(
                message.message_id(),
                message.sender.into(),
                message.recipient.into(),
                message.amount,
                message.nonce,
                0,
                message.data,
            ),
        })
        .collect()
}

pub(crate) fn generate_contract_inputs(contract_ids: HashSet<ContractId>) -> Vec<Input> {
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

/// Execute the transaction in a simulated manner, not modifying blockchain state
pub async fn simulate_and_check_success<T: Transaction + Clone>(
    provider: &Provider,
    tx: &T,
) -> Result<Vec<Receipt>> {
    let receipts = provider.dry_run(tx).await?;
    has_script_succeeded(&receipts)?;

    Ok(receipts)
}

fn has_script_succeeded(receipts: &[Receipt]) -> Result<()> {
    receipts
        .iter()
        .find_map(|receipt| match receipt {
            Receipt::ScriptResult { result, .. } if *result != ScriptExecutionResult::Success => {
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

#[cfg(test)]
mod test {
    use std::slice;

    use fuels_core::abi_encoder::ABIEncoder;
    use fuels_types::{
        bech32::Bech32ContractId,
        coin::{Coin, CoinStatus},
        param_types::ParamType,
        parameters::CallParameters,
        Token,
    };
    use rand::Rng;

    use super::*;

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
                output_param: ParamType::Unit,
                message_outputs: None,
                is_payable: false,
                custom_assets: Default::default(),
            }
        }
    }

    fn random_bech32_addr() -> Bech32Address {
        Bech32Address::new("fuel", rand::thread_rng().gen::<[u8; 32]>())
    }

    fn random_bech32_contract_id() -> Bech32ContractId {
        Bech32ContractId::new("fuel", rand::thread_rng().gen::<[u8; 32]>())
    }

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

        // Call 2 has multiple inputs, compute_custom_input_offset will be true

        let args = [Token::U8(1), Token::U16(2), Token::U8(3)]
            .map(|token| ABIEncoder::encode(&[token]).unwrap())
            .to_vec();

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
                message_outputs: None,
                external_contracts: vec![],
                output_param: ParamType::Unit,
                is_payable: false,
                custom_assets: Default::default(),
            })
            .collect();

        // Act
        let (script_data, param_offsets) = build_script_data_from_contract_calls(&calls, 0, 0);

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
        assert_eq!(call_1_arg, args[0].resolve(0));

        let call_3_arg_offset = param_offsets[2].call_data_offset + ContractId::LEN + SELECTOR_LEN;
        let call_3_arg = script_data[call_3_arg_offset..call_3_arg_offset + WORD_SIZE].to_vec();
        assert_eq!(call_3_arg, args[2].resolve(0));

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
            script_data[custom_input_offset..custom_input_offset + WORD_SIZE].to_vec();
        assert_eq!(custom_input, args[1].resolve(0));
    }

    #[test]
    fn contract_input_present() {
        let call = ContractCall::new_with_random_id();

        let (inputs, _) = get_transaction_inputs_outputs(
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

        let (inputs, _) =
            get_transaction_inputs_outputs(&calls, &random_bech32_addr(), Default::default());

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

        let (_, outputs) =
            get_transaction_inputs_outputs(&[call], &random_bech32_addr(), Default::default());

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
        let (inputs, _) = get_transaction_inputs_outputs(
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
        let (_, outputs) =
            get_transaction_inputs_outputs(&[call], &random_bech32_addr(), Default::default());

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
            .map(|asset_id| {
                Resource::Coin(Coin {
                    amount: 100,
                    block_created: 0,
                    asset_id,
                    utxo_id: Default::default(),
                    maturity: 0,
                    owner: Default::default(),
                    status: CoinStatus::Unspent,
                })
            })
            .collect();
        let call = ContractCall::new_with_random_id();

        // when
        let (_, outputs) = get_transaction_inputs_outputs(&[call], &wallet_addr, coins);

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

        let generate_spendable_resources = || {
            asset_ids
                .into_iter()
                .enumerate()
                .map(|(index, asset_id)| {
                    Resource::Coin(Coin {
                        amount: (index * 10) as u64,
                        block_created: 1,
                        asset_id,
                        utxo_id: Default::default(),
                        maturity: 0,
                        owner: Default::default(),
                        status: CoinStatus::Unspent,
                    })
                })
                .collect::<Vec<_>>()
        };

        let call = ContractCall::new_with_random_id();

        // when
        let (inputs, _) = get_transaction_inputs_outputs(
            &[call],
            &random_bech32_addr(),
            generate_spendable_resources(),
        );

        // then
        let inputs_as_signed_coins: HashSet<Input> = inputs[1..].iter().cloned().collect();

        let expected_inputs = generate_spendable_resources()
            .into_iter()
            .map(|resource| match resource {
                Resource::Coin(coin) => Input::coin_signed(
                    coin.utxo_id,
                    coin.owner.into(),
                    coin.amount,
                    coin.asset_id,
                    TxPointer::default(),
                    0,
                    0,
                ),
                Resource::Message(_) => panic!("Resources contained messages."),
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
        let (_, outputs) =
            get_transaction_inputs_outputs(&calls, &random_bech32_addr(), Default::default());

        // then
        let actual_variable_outputs: HashSet<Output> = outputs[2..].iter().cloned().collect();
        let expected_outputs: HashSet<Output> = variable_outputs.into();

        assert_eq!(expected_outputs, actual_variable_outputs);
    }

    #[test]
    fn message_outputs_appended_to_outputs() {
        // given
        let message_outputs =
            [100, 200].map(|amount| Output::message(random_bech32_addr().into(), amount));

        let calls = message_outputs
            .iter()
            .cloned()
            .map(|message_output| {
                ContractCall::new_with_random_id().with_message_outputs(vec![message_output])
            })
            .collect::<Vec<_>>();

        // when
        let (_, outputs) =
            get_transaction_inputs_outputs(&calls, &random_bech32_addr(), Default::default());

        // then
        let actual_message_outputs: HashSet<Output> = outputs[2..].iter().cloned().collect();
        let expected_outputs: HashSet<Output> = message_outputs.into();

        assert_eq!(expected_outputs, actual_message_outputs);
    }

    #[test]
    fn will_collate_same_asset_ids() {
        let asset_id_1 = AssetId::from([1; 32]);
        let asset_id_2 = AssetId::from([2; 32]);

        let calls = [
            (asset_id_1, 100),
            (asset_id_2, 200),
            (asset_id_1, 300),
            (asset_id_2, 400),
        ]
        .map(|(asset_id, amount)| CallParameters::new(Some(amount), Some(asset_id), None))
        .map(|call_parameters| {
            ContractCall::new_with_random_id().with_call_parameters(call_parameters)
        });

        let asset_id_amounts = calculate_required_asset_amounts(&calls);

        let expected_asset_id_amounts = [(asset_id_1, 400), (asset_id_2, 600)].into();

        assert_eq!(
            asset_id_amounts.into_iter().collect::<HashSet<_>>(),
            expected_asset_id_amounts
        )
    }
}
