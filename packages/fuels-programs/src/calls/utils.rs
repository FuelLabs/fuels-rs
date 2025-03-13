use std::{collections::HashSet, iter, vec};

use fuel_abi_types::error_codes::FAILED_TRANSFER_TO_ADDRESS_SIGNAL;
use fuel_asm::{op, RegId};
use fuel_tx::{AssetId, Bytes32, ContractId, Output, PanicReason, Receipt, TxPointer, UtxoId};
use fuels_accounts::Account;
use fuels_core::{
    offsets::call_script_data_offset,
    types::{
        bech32::{Bech32Address, Bech32ContractId},
        errors::Result,
        input::Input,
        transaction::{ScriptTransaction, TxPolicies},
        transaction_builders::{
            BuildableTransaction, ScriptTransactionBuilder, TransactionBuilder,
            VariableOutputPolicy,
        },
    },
};
use itertools::{chain, Itertools};

use crate::{
    assembly::contract_call::{CallOpcodeParamsOffset, ContractCallInstructions},
    calls::ContractCall,
    DEFAULT_MAX_FEE_ESTIMATION_TOLERANCE,
};

pub(crate) mod sealed {
    pub trait Sealed {}
}

/// Creates a [`ScriptTransactionBuilder`] from contract calls.
pub(crate) async fn transaction_builder_from_contract_calls(
    calls: &[ContractCall],
    tx_policies: TxPolicies,
    variable_outputs: VariableOutputPolicy,
    account: &impl Account,
) -> Result<ScriptTransactionBuilder> {
    let calls_instructions_len = compute_calls_instructions_len(calls);
    let provider = account.try_provider()?;
    let consensus_parameters = provider.consensus_parameters().await?;
    let data_offset = call_script_data_offset(&consensus_parameters, calls_instructions_len)?;

    let (script_data, call_param_offsets) = build_script_data_from_contract_calls(
        calls,
        data_offset,
        *consensus_parameters.base_asset_id(),
    )?;
    let script = get_instructions(call_param_offsets);

    let required_asset_amounts =
        calculate_required_asset_amounts(calls, *consensus_parameters.base_asset_id());

    // Find the spendable resources required for those calls
    let mut asset_inputs = vec![];
    for (asset_id, amount) in &required_asset_amounts {
        let resources = account
            .get_asset_inputs_for_amount(*asset_id, *amount, None)
            .await?;
        asset_inputs.extend(resources);
    }

    let (inputs, outputs) = get_transaction_inputs_outputs(
        calls,
        asset_inputs,
        account.address(),
        *consensus_parameters.base_asset_id(),
    );

    Ok(ScriptTransactionBuilder::default()
        .with_variable_output_policy(variable_outputs)
        .with_tx_policies(tx_policies)
        .with_script(script)
        .with_script_data(script_data.clone())
        .with_inputs(inputs)
        .with_outputs(outputs)
        .with_gas_estimation_tolerance(DEFAULT_MAX_FEE_ESTIMATION_TOLERANCE)
        .with_max_fee_estimation_tolerance(DEFAULT_MAX_FEE_ESTIMATION_TOLERANCE))
}

/// Creates a [`ScriptTransaction`] from contract calls. The internal [Transaction] is
/// initialized with the actual script instructions, script data needed to perform the call and
/// transaction inputs/outputs consisting of assets and contracts.
pub(crate) async fn build_with_tb(
    calls: &[ContractCall],
    mut tb: ScriptTransactionBuilder,
    account: &impl Account,
) -> Result<ScriptTransaction> {
    let consensus_parameters = account.try_provider()?.consensus_parameters().await?;
    let base_asset_id = *consensus_parameters.base_asset_id();
    let required_asset_amounts = calculate_required_asset_amounts(calls, base_asset_id);

    let used_base_amount = required_asset_amounts
        .iter()
        .find_map(|(asset_id, amount)| (*asset_id == base_asset_id).then_some(*amount))
        .unwrap_or_default();

    account.add_witnesses(&mut tb)?;
    account.adjust_for_fee(&mut tb, used_base_amount).await?;

    tb.build(account.try_provider()?).await
}

/// Compute the length of the calling scripts for the two types of contract calls: those that return
/// a heap type, and those that don't.
fn compute_calls_instructions_len(calls: &[ContractCall]) -> usize {
    calls
        .iter()
        .map(|c| {
            // Use placeholder for `call_param_offsets` and `output_param_type`, because the length of
            // the calling script doesn't depend on the underlying type, just on whether or not
            // gas was forwarded.
            let call_opcode_params = CallOpcodeParamsOffset {
                gas_forwarded_offset: c.call_parameters.gas_forwarded().map(|_| 0),
                ..CallOpcodeParamsOffset::default()
            };

            ContractCallInstructions::new(call_opcode_params)
                .into_bytes()
                .count()
        })
        .sum()
}

/// Compute how much of each asset is required based on all `CallParameters` of the `ContractCalls`
pub(crate) fn calculate_required_asset_amounts(
    calls: &[ContractCall],
    base_asset_id: AssetId,
) -> Vec<(AssetId, u64)> {
    let call_param_assets = calls.iter().map(|call| {
        (
            call.call_parameters.asset_id().unwrap_or(base_asset_id),
            call.call_parameters.amount(),
        )
    });

    let grouped_assets = calls
        .iter()
        .flat_map(|call| call.custom_assets.clone())
        .map(|((asset_id, _), amount)| (asset_id, amount))
        .chain(call_param_assets)
        .sorted_by_key(|(asset_id, _)| *asset_id)
        .group_by(|(asset_id, _)| *asset_id);

    grouped_assets
        .into_iter()
        .filter_map(|(asset_id, groups_w_same_asset_id)| {
            let total_amount_in_group = groups_w_same_asset_id.map(|(_, amount)| amount).sum();

            (total_amount_in_group != 0).then_some((asset_id, total_amount_in_group))
        })
        .collect()
}

/// Given a list of contract calls, create the actual opcodes used to call the contract
pub(crate) fn get_instructions(offsets: Vec<CallOpcodeParamsOffset>) -> Vec<u8> {
    offsets
        .into_iter()
        .flat_map(|offset| ContractCallInstructions::new(offset).into_bytes())
        .chain(op::ret(RegId::ONE).to_bytes())
        .collect()
}

pub(crate) fn build_script_data_from_contract_calls(
    calls: &[ContractCall],
    data_offset: usize,
    base_asset_id: AssetId,
) -> Result<(Vec<u8>, Vec<CallOpcodeParamsOffset>)> {
    calls.iter().try_fold(
        (vec![], vec![]),
        |(mut script_data, mut param_offsets), call| {
            let segment_offset = data_offset + script_data.len();
            let offset = call
                .data(base_asset_id)?
                .encode(segment_offset, &mut script_data);

            param_offsets.push(offset);
            Ok((script_data, param_offsets))
        },
    )
}

/// Returns the assets and contracts that will be consumed ([`Input`]s)
/// and created ([`Output`]s) by the transaction
pub(crate) fn get_transaction_inputs_outputs(
    calls: &[ContractCall],
    asset_inputs: Vec<Input>,
    address: &Bech32Address,
    base_asset_id: AssetId,
) -> (Vec<Input>, Vec<Output>) {
    let asset_ids = extract_unique_asset_ids(&asset_inputs, base_asset_id);
    let contract_ids = extract_unique_contract_ids(calls);
    let num_of_contracts = contract_ids.len();

    // Custom `Inputs` and `Outputs` should be placed before other inputs and outputs.
    let custom_inputs = calls.iter().flat_map(|c| c.inputs.clone()).collect_vec();
    let custom_inputs_len = custom_inputs.len();
    let custom_outputs = calls.iter().flat_map(|c| c.outputs.clone()).collect_vec();

    let inputs = chain!(
        custom_inputs,
        generate_contract_inputs(contract_ids, custom_outputs.len()),
        asset_inputs
    )
    .collect();

    // Note the contract_outputs are placed after the custom outputs and
    // the contract_inputs are referencing them via `output_index`. The
    // node will, upon receiving our request, use `output_index` to index
    // the `inputs` array we've sent over.
    let outputs = chain!(
        custom_outputs,
        generate_contract_outputs(num_of_contracts, custom_inputs_len),
        generate_asset_change_outputs(address, asset_ids),
        generate_custom_outputs(calls),
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

fn extract_unique_asset_ids(asset_inputs: &[Input], base_asset_id: AssetId) -> HashSet<AssetId> {
    asset_inputs
        .iter()
        .filter_map(|input| match input {
            Input::ResourceSigned { resource, .. } | Input::ResourcePredicate { resource, .. } => {
                Some(resource.coin_asset_id().unwrap_or(base_asset_id))
            }
            _ => None,
        })
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

/// Generate contract outputs taking in consideration already existing inputs
pub(crate) fn generate_contract_outputs(
    num_of_contracts: usize,
    num_current_inputs: usize,
) -> Vec<Output> {
    (0..num_of_contracts)
        .map(|idx| {
            Output::contract(
                (idx + num_current_inputs) as u16,
                Bytes32::zeroed(),
                Bytes32::zeroed(),
            )
        })
        .collect()
}

/// Generate contract inputs taking in consideration already existing outputs
pub(crate) fn generate_contract_inputs(
    contract_ids: HashSet<ContractId>,
    num_current_outputs: usize,
) -> Vec<Input> {
    contract_ids
        .into_iter()
        .enumerate()
        .map(|(idx, contract_id)| {
            Input::contract(
                UtxoId::new(Bytes32::zeroed(), (idx + num_current_outputs) as u16),
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

pub fn is_missing_output_variables(receipts: &[Receipt]) -> bool {
    receipts.iter().any(
        |r| matches!(r, Receipt::Revert { ra, .. } if *ra == FAILED_TRANSFER_TO_ADDRESS_SIGNAL),
    )
}

pub fn find_id_of_missing_contract(receipts: &[Receipt]) -> Option<Bech32ContractId> {
    receipts.iter().find_map(|receipt| match receipt {
        Receipt::Panic {
            reason,
            contract_id,
            ..
        } if *reason.reason() == PanicReason::ContractNotInInputs => {
            let contract_id = contract_id
                .expect("panic caused by a contract not in inputs must have a contract id");
            Some(Bech32ContractId::from(contract_id))
        }
        _ => None,
    })
}

#[cfg(test)]
mod test {
    use std::slice;

    use fuels_accounts::signers::private_key::PrivateKeySigner;
    use fuels_core::types::{
        coin::{Coin, CoinStatus},
        coin_type::CoinType,
        param_types::ParamType,
    };
    use rand::{thread_rng, Rng};

    use super::*;
    use crate::calls::{traits::ContractDependencyConfigurator, CallParameters};

    fn new_contract_call_with_random_id() -> ContractCall {
        ContractCall {
            contract_id: random_bech32_contract_id(),
            encoded_args: Ok(Default::default()),
            encoded_selector: [0; 8].to_vec(),
            call_parameters: Default::default(),
            external_contracts: Default::default(),
            output_param: ParamType::Unit,
            is_payable: false,
            custom_assets: Default::default(),
            inputs: vec![],
            outputs: vec![],
        }
    }

    fn random_bech32_contract_id() -> Bech32ContractId {
        Bech32ContractId::new("fuel", rand::thread_rng().gen::<[u8; 32]>())
    }

    #[test]
    fn contract_input_present() {
        let call = new_contract_call_with_random_id();

        let signer = PrivateKeySigner::random(&mut thread_rng());

        let (inputs, _) = get_transaction_inputs_outputs(
            slice::from_ref(&call),
            Default::default(),
            signer.address(),
            AssetId::zeroed(),
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
        let call = new_contract_call_with_random_id();
        let call_w_same_contract =
            new_contract_call_with_random_id().with_contract_id(call.contract_id.clone());

        let signer = PrivateKeySigner::random(&mut thread_rng());

        let calls = [call, call_w_same_contract];

        let (inputs, _) = get_transaction_inputs_outputs(
            &calls,
            Default::default(),
            signer.address(),
            AssetId::zeroed(),
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
        let call = new_contract_call_with_random_id();

        let signer = PrivateKeySigner::random(&mut thread_rng());

        let (_, outputs) = get_transaction_inputs_outputs(
            &[call],
            Default::default(),
            signer.address(),
            AssetId::zeroed(),
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
        let call = new_contract_call_with_random_id()
            .with_external_contracts(vec![external_contract_id.clone()]);

        let signer = PrivateKeySigner::random(&mut thread_rng());

        // when
        let (inputs, _) = get_transaction_inputs_outputs(
            slice::from_ref(&call),
            Default::default(),
            signer.address(),
            AssetId::zeroed(),
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
                    assert_eq!(utxo_id, UtxoId::new(Bytes32::zeroed(), index as u16));
                    assert_eq!(balance_root, Bytes32::zeroed());
                    assert_eq!(state_root, Bytes32::zeroed());
                    assert_eq!(tx_pointer, TxPointer::default());
                    assert!(expected_contract_ids.contains(&contract_id));
                    expected_contract_ids.remove(&contract_id);
                }
                _ => {
                    panic!("expected only inputs of type `Input::Contract`");
                }
            }
        }
    }

    #[test]
    fn external_contract_output_present() {
        // given
        let external_contract_id = random_bech32_contract_id();
        let call =
            new_contract_call_with_random_id().with_external_contracts(vec![external_contract_id]);

        let signer = PrivateKeySigner::random(&mut thread_rng());

        // when
        let (_, outputs) = get_transaction_inputs_outputs(
            &[call],
            Default::default(),
            signer.address(),
            AssetId::zeroed(),
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
        let asset_ids = [AssetId::zeroed(), AssetId::from([1; 32])];

        let coins = asset_ids
            .into_iter()
            .map(|asset_id| {
                let coin = CoinType::Coin(Coin {
                    amount: 100,
                    block_created: 0u32,
                    asset_id,
                    utxo_id: Default::default(),
                    owner: Default::default(),
                    status: CoinStatus::Unspent,
                });
                Input::resource_signed(coin)
            })
            .collect();
        let call = new_contract_call_with_random_id();

        let signer = PrivateKeySigner::random(&mut thread_rng());

        // when
        let (_, outputs) =
            get_transaction_inputs_outputs(&[call], coins, signer.address(), AssetId::zeroed());

        // then
        let change_outputs: HashSet<Output> = outputs[1..].iter().cloned().collect();

        let expected_change_outputs = asset_ids
            .into_iter()
            .map(|asset_id| Output::Change {
                to: signer.address().into(),
                amount: 0,
                asset_id,
            })
            .collect();

        assert_eq!(change_outputs, expected_change_outputs);
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
        .map(|(asset_id, amount)| {
            CallParameters::default()
                .with_amount(amount)
                .with_asset_id(asset_id)
        })
        .map(|call_parameters| {
            new_contract_call_with_random_id().with_call_parameters(call_parameters)
        });

        let asset_id_amounts = calculate_required_asset_amounts(&calls, AssetId::zeroed());

        let expected_asset_id_amounts = [(asset_id_1, 400), (asset_id_2, 600)].into();

        assert_eq!(
            asset_id_amounts.into_iter().collect::<HashSet<_>>(),
            expected_asset_id_amounts
        )
    }

    mod compute_calls_instructions_len {
        use fuel_asm::Instruction;
        use fuels_core::types::param_types::{EnumVariants, ParamType};

        use super::new_contract_call_with_random_id;
        use crate::calls::utils::compute_calls_instructions_len;

        // movi, movi, lw, movi + call (for gas)
        const BASE_INSTRUCTION_COUNT: usize = 5;
        // 2 instructions (movi and lw) added in get_single_call_instructions when gas_offset is set
        const GAS_OFFSET_INSTRUCTION_COUNT: usize = 2;

        #[test]
        fn test_simple() {
            let call = new_contract_call_with_random_id();
            let instructions_len = compute_calls_instructions_len(&[call]);
            assert_eq!(instructions_len, Instruction::SIZE * BASE_INSTRUCTION_COUNT);
        }

        #[test]
        fn test_with_gas_offset() {
            let mut call = new_contract_call_with_random_id();
            call.call_parameters = call.call_parameters.with_gas_forwarded(0);
            let instructions_len = compute_calls_instructions_len(&[call]);
            assert_eq!(
                instructions_len,
                Instruction::SIZE * (BASE_INSTRUCTION_COUNT + GAS_OFFSET_INSTRUCTION_COUNT)
            );
        }

        #[test]
        fn test_with_enum_with_only_non_heap_variants() {
            let mut call = new_contract_call_with_random_id();
            call.output_param = ParamType::Enum {
                name: "".to_string(),
                enum_variants: EnumVariants::new(vec![
                    ("".to_string(), ParamType::Bool),
                    ("".to_string(), ParamType::U8),
                ])
                .unwrap(),
                generics: Vec::new(),
            };
            let instructions_len = compute_calls_instructions_len(&[call]);
            assert_eq!(
                instructions_len,
                // no extra instructions if there are no heap type variants
                Instruction::SIZE * BASE_INSTRUCTION_COUNT
            );
        }
    }
}
