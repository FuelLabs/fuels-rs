use fuel_gql_client::fuel_tx::{Input, Output, TxPointer, UtxoId};
use fuel_gql_client::fuel_types::{Immediate18, Word};
use fuel_gql_client::fuel_vm::{consts::REG_ONE, prelude::Opcode};
use fuel_tx::{AssetId, Bytes32, ContractId};
use fuels_core::constants::BASE_ASSET_ID;
use fuels_types::bech32::Bech32Address;
use fuels_types::constants::WORD_SIZE;
use fuels_types::resource::Resource;
use itertools::Itertools;
use std::collections::HashSet;
use std::{iter, vec};

use crate::contract::ContractCall;

#[derive(Default)]
/// Specifies offsets of [`Opcode::CALL`] parameters stored in the script
/// data from which they can be loaded into registers
pub(crate) struct CallOpcodeParamsOffset {
    pub asset_id_offset: usize,
    pub amount_offset: usize,
    pub gas_forwarded_offset: usize,
    pub call_data_offset: usize,
}

/// Compute how much of each asset is required based on all `CallParameters` of the `ContractCalls`
pub(crate) fn calculate_required_asset_amounts(calls: &[ContractCall]) -> Vec<(AssetId, u64)> {
    let amounts_per_asset_id = calls
        .iter()
        .map(|call| (call.call_parameters.asset_id, call.call_parameters.amount))
        .collect::<Vec<_>>();
    sum_up_amounts_for_each_asset_id(amounts_per_asset_id)
}

/// Sum up the amounts required in each call for each asset ID, so you can get a total for each
/// asset over all calls.
pub fn sum_up_amounts_for_each_asset_id(
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

    instructions.extend(Opcode::RET(REG_ONE).to_bytes());

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

pub fn extract_unique_asset_ids(spendable_coins: &[Resource]) -> HashSet<AssetId> {
    spendable_coins
        .iter()
        .map(|resource| match resource {
            Resource::Coin(coin) => coin.asset_id,
            Resource::Message(_) => BASE_ASSET_ID,
        })
        .collect()
}

pub fn extract_variable_outputs(calls: &[ContractCall]) -> Vec<Output> {
    calls
        .iter()
        .filter_map(|call| call.variable_outputs.clone())
        .flatten()
        .collect()
}

pub fn extract_message_outputs(calls: &[ContractCall]) -> Vec<Output> {
    calls
        .iter()
        .filter_map(|call| call.message_outputs.clone())
        .flatten()
        .collect()
}

pub fn generate_asset_change_outputs(
    wallet_address: &Bech32Address,
    asset_ids: HashSet<AssetId>,
) -> Vec<Output> {
    asset_ids
        .into_iter()
        .map(|asset_id| Output::change(wallet_address.into(), 0, asset_id))
        .collect()
}

pub fn generate_contract_outputs(num_of_contracts: usize) -> Vec<Output> {
    (0..num_of_contracts)
        .map(|idx| Output::contract(idx as u8, Bytes32::zeroed(), Bytes32::zeroed()))
        .collect()
}

pub fn convert_to_signed_resources(spendable_resources: Vec<Resource>) -> Vec<Input> {
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

pub fn generate_contract_inputs(contract_ids: HashSet<ContractId>) -> Vec<Input> {
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

pub fn extract_unique_contract_ids(calls: &[ContractCall]) -> HashSet<ContractId> {
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

#[cfg(test)]
mod test {
    use super::*;
    use fuels_core::abi_encoder::ABIEncoder;
    use fuels_core::parameters::CallParameters;
    use fuels_core::Token;
    use fuels_types::bech32::Bech32ContractId;
    use fuels_types::param_types::ParamType;
    use rand::Rng;

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
            }
        }
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
