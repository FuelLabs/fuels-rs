use std::{collections::HashSet, iter, vec};

use fuel_abi_types::error_codes::FAILED_TRANSFER_TO_ADDRESS_SIGNAL;
use fuel_asm::{op, RegId};
use fuel_tx::{AssetId, Bytes32, ContractId, Output, PanicReason, Receipt, TxPointer, UtxoId};
use fuel_types::{Address, Word};
use fuels_accounts::Account;
use fuels_core::{
    constants::WORD_SIZE,
    offsets::call_script_data_offset,
    types::{
        bech32::{Bech32Address, Bech32ContractId},
        errors::{Error as FuelsError, Result},
        input::Input,
        param_types::ParamType,
        transaction::{ScriptTransaction, TxParameters},
        transaction_builders::ScriptTransactionBuilder,
    },
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

/// How many times to attempt to resolve missing tx dependencies.
pub const DEFAULT_TX_DEP_ESTIMATION_ATTEMPTS: u64 = 10;

#[async_trait::async_trait]
pub trait TxDependencyExtension: Sized {
    async fn simulate(&mut self) -> Result<()>;

    /// Appends `num` [`fuel_tx::Output::Variable`]s to the transaction.
    /// Note that this is a builder method, i.e. use it as a chain:
    ///
    /// ```ignore
    /// my_contract_instance.my_method(...).append_variable_outputs(num).call()
    /// my_script_instance.main(...).append_variable_outputs(num).call()
    /// ```
    ///
    /// [`Output::Variable`]: fuel_tx::Output::Variable
    fn append_variable_outputs(self, num: u64) -> Self;

    /// Appends additional external contracts as dependencies to this call.
    /// Effectively, this will be used to create additional
    /// [`fuel_tx::Input::Contract`]/[`fuel_tx::Output::Contract`]
    /// pairs and set them into the transaction. Note that this is a builder
    /// method, i.e. use it as a chain:
    ///
    /// ```ignore
    /// my_contract_instance.my_method(...).append_contract(additional_contract_id).call()
    /// my_script_instance.main(...).append_contract(additional_contract_id).call()
    /// ```
    ///
    /// [`Input::Contract`]: fuel_tx::Input::Contract
    /// [`Output::Contract`]: fuel_tx::Output::Contract
    fn append_contract(self, contract_id: Bech32ContractId) -> Self;

    fn append_missing_dependencies(mut self, receipts: &[Receipt]) -> Self {
        if is_missing_output_variables(receipts) {
            self = self.append_variable_outputs(1);
        }
        if let Some(contract_id) = find_id_of_missing_contract(receipts) {
            self = self.append_contract(contract_id);
        }

        self
    }

    /// Simulates the call and attempts to resolve missing tx dependencies.
    /// Forwards the received error if it cannot be fixed.
    async fn estimate_tx_dependencies(mut self, max_attempts: Option<u64>) -> Result<Self> {
        let attempts = max_attempts.unwrap_or(DEFAULT_TX_DEP_ESTIMATION_ATTEMPTS);

        for _ in 0..attempts {
            match self.simulate().await {
                Ok(_) => return Ok(self),

                Err(FuelsError::RevertTransactionError { ref receipts, .. }) => {
                    self = self.append_missing_dependencies(receipts);
                }

                Err(other_error) => return Err(other_error),
            }
        }

        self.simulate().await.map(|_| self)
    }
}

/// Creates a [`ScriptTransaction`] from contract calls. The internal [Transaction] is
/// initialized with the actual script instructions, script data needed to perform the call and
/// transaction inputs/outputs consisting of assets and contracts.
pub(crate) async fn build_tx_from_contract_calls(
    calls: &[ContractCall],
    tx_parameters: TxParameters,
    account: &impl Account,
) -> Result<ScriptTransaction> {
    let consensus_parameters = account.try_provider()?.consensus_parameters();

    let calls_instructions_len = compute_calls_instructions_len(calls);
    let data_offset = call_script_data_offset(&consensus_parameters, calls_instructions_len);

    let (script_data, call_param_offsets) =
        build_script_data_from_contract_calls(calls, data_offset, tx_parameters.gas_limit());

    let script = get_instructions(calls, call_param_offsets);

    let required_asset_amounts = calculate_required_asset_amounts(calls);

    let mut asset_inputs = vec![];

    // Find the spendable resources required for those calls
    for (asset_id, amount) in &required_asset_amounts {
        let resources = account
            .get_asset_inputs_for_amount(*asset_id, *amount)
            .await?;
        asset_inputs.extend(resources);
    }

    let (inputs, outputs) = get_transaction_inputs_outputs(calls, asset_inputs, account);

    let tb = ScriptTransactionBuilder::prepare_transfer(inputs, outputs, tx_parameters)
        .with_script(script)
        .with_script_data(script_data.clone());

    let base_asset_amount = required_asset_amounts
        .iter()
        .find_map(|(asset_id, amount)| (*asset_id == AssetId::default()).then_some(*amount))
        .unwrap_or_default();

    account.add_fee_resources(tb, base_asset_amount).await
}

/// Compute the length of the calling scripts for the two types of contract calls: those that return
/// a heap type, and those that don't.
fn compute_calls_instructions_len(calls: &[ContractCall]) -> usize {
    let n_heap_type_calls = calls
        .iter()
        .filter(|c| c.output_param.is_vm_heap_type())
        .count();
    let n_stack_type_calls = calls.len() - n_heap_type_calls;

    let total_instructions_len_stack_data =
        // Use placeholder for `call_param_offsets` and `output_param_type`, because the length of
        // the calling script doesn't depend on the underlying type, just on whether or not the
        // contract call output type is a heap type.
        get_single_call_instructions(&CallOpcodeParamsOffset::default(), &ParamType::U64).len()
            * n_stack_type_calls;

    let total_instructions_len_heap_data = get_single_call_instructions(
        &CallOpcodeParamsOffset::default(),
        &ParamType::Vector(Box::from(ParamType::U64)),
    )
    .len()
        * n_heap_type_calls;

    total_instructions_len_stack_data + total_instructions_len_heap_data
}

/// Compute how much of each asset is required based on all `CallParameters` of the `ContractCalls`
pub(crate) fn calculate_required_asset_amounts(calls: &[ContractCall]) -> Vec<(AssetId, u64)> {
    let call_param_assets = calls
        .iter()
        .map(|call| {
            (
                call.call_parameters.asset_id(),
                call.call_parameters.amount(),
            )
        })
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
    calls
        .iter()
        .zip(&offsets)
        .flat_map(|(call, offset)| get_single_call_instructions(offset, &call.output_param))
        .chain(op::ret(RegId::ONE).to_bytes())
        .collect()
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

        script_data.extend(call.call_parameters.asset_id().iter());

        script_data.extend(call.call_parameters.amount().to_be_bytes());

        // If gas_forwarded is not set, use the transaction gas limit
        let gas_forwarded = call.call_parameters.gas_forwarded().unwrap_or(gas_limit);
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
pub(crate) fn get_single_call_instructions(
    offsets: &CallOpcodeParamsOffset,
    output_param_type: &ParamType,
) -> Vec<u8> {
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

    let mut instructions = [
        op::movi(0x10, call_data_offset),
        op::movi(0x11, gas_forwarded_offset),
        op::lw(0x11, 0x11, 0),
        op::movi(0x12, amount_offset),
        op::lw(0x12, 0x12, 0),
        op::movi(0x13, asset_id_offset),
        op::call(0x10, 0x12, 0x13, 0x11),
    ]
    .to_vec();
    // The instructions are different if you want to return data that was on the heap
    if let Some(inner_type_byte_size) = output_param_type.heap_inner_element_size() {
        instructions.extend([
            // The RET register contains the pointer address of the `CALL` return (a stack
            // address).
            // The RETL register contains the length of the `CALL` return (=24 because the
            // Vec/Bytes/String struct takes 3 WORDs).
            // We don't actually need it unless the Vec/Bytes/String struct encoding changes in the
            // compiler.
            // Load the word located at the address contained in RET, it's a word that
            // translates to a heap address. 0x15 is a free register.
            op::lw(0x15, RegId::RET, 0),
            // We know a Vec/Bytes/String struct has its third WORD contain the length of the
            // underlying vector, so use a 2 offset to store the length in 0x16, which is a free
            // register.
            op::lw(0x16, RegId::RET, 2),
            // The in-memory size of the type is (in-memory size of the inner type) * length
            op::muli(0x16, 0x16, inner_type_byte_size as u16),
            op::retd(0x15, 0x16),
        ]);
    }

    #[allow(clippy::iter_cloned_collect)]
    instructions.into_iter().collect::<Vec<u8>>()
}

/// Returns the assets and contracts that will be consumed ([`Input`]s)
/// and created ([`Output`]s) by the transaction
pub(crate) fn get_transaction_inputs_outputs(
    calls: &[ContractCall],
    asset_inputs: Vec<Input>,
    account: &impl Account,
) -> (Vec<Input>, Vec<Output>) {
    let asset_ids = extract_unique_asset_ids(&asset_inputs);
    let contract_ids = extract_unique_contract_ids(calls);
    let num_of_contracts = contract_ids.len();

    let inputs = chain!(generate_contract_inputs(contract_ids), asset_inputs).collect();

    // Note the contract_outputs need to come first since the
    // contract_inputs are referencing them via `output_index`. The node
    // will, upon receiving our request, use `output_index` to index the
    // `inputs` array we've sent over.
    let outputs = chain!(
        generate_contract_outputs(num_of_contracts),
        generate_asset_change_outputs(account.address(), asset_ids),
        generate_custom_outputs(calls),
        extract_variable_outputs(calls)
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

fn extract_unique_asset_ids(asset_inputs: &[Input]) -> HashSet<AssetId> {
    asset_inputs
        .iter()
        .filter_map(|input| match input {
            Input::ResourceSigned { resource, .. } | Input::ResourcePredicate { resource, .. } => {
                Some(resource.asset_id())
            }
            _ => None,
        })
        .collect()
}

fn extract_variable_outputs(calls: &[ContractCall]) -> Vec<Output> {
    calls
        .iter()
        .flat_map(|call| call.variable_outputs.clone())
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

pub fn new_variable_outputs(num: usize) -> Vec<Output> {
    vec![
        Output::Variable {
            amount: 0,
            to: Address::zeroed(),
            asset_id: AssetId::default(),
        };
        num
    ]
}

#[cfg(test)]
mod test {
    use std::slice;

    use fuels_accounts::wallet::WalletUnlocked;
    use fuels_core::{
        codec::ABIEncoder,
        types::{
            bech32::Bech32ContractId,
            coin::{Coin, CoinStatus},
            coin_type::CoinType,
            Token,
        },
    };
    use rand::Rng;

    use super::*;
    use crate::contract::CallParameters;

    impl ContractCall {
        pub fn new_with_random_id() -> Self {
            ContractCall {
                contract_id: random_bech32_contract_id(),
                encoded_args: Default::default(),
                encoded_selector: [0; 8],
                call_parameters: Default::default(),
                compute_custom_input_offset: false,
                variable_outputs: vec![],
                external_contracts: Default::default(),
                output_param: ParamType::Unit,
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

        let asset_ids = [
            AssetId::from([4u8; 32]),
            AssetId::from([5u8; 32]),
            AssetId::from([6u8; 32]),
        ];

        let selectors = [[7u8; 8], [8u8; 8], [9u8; 8]];

        // Call 2 has multiple inputs, compute_custom_input_offset will be true

        let args = [Token::U8(1), Token::U16(2), Token::U8(3)]
            .map(|token| ABIEncoder::encode(&[token]).unwrap())
            .to_vec();

        let calls: Vec<ContractCall> = (0..NUM_CALLS)
            .map(|i| ContractCall {
                contract_id: contract_ids[i].clone(),
                encoded_selector: selectors[i],
                encoded_args: args[i].clone(),
                call_parameters: CallParameters::new(i as u64, asset_ids[i], i as u64),
                compute_custom_input_offset: i == 1,
                variable_outputs: vec![],
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

        let wallet = WalletUnlocked::new_random(None);

        let (inputs, _) =
            get_transaction_inputs_outputs(slice::from_ref(&call), Default::default(), &wallet);

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

        let wallet = WalletUnlocked::new_random(None);

        let calls = [call, call_w_same_contract];

        let (inputs, _) = get_transaction_inputs_outputs(&calls, Default::default(), &wallet);

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

        let wallet = WalletUnlocked::new_random(None);

        let (_, outputs) = get_transaction_inputs_outputs(&[call], Default::default(), &wallet);

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

        let wallet = WalletUnlocked::new_random(None);

        // when
        let (inputs, _) =
            get_transaction_inputs_outputs(slice::from_ref(&call), Default::default(), &wallet);

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

        let wallet = WalletUnlocked::new_random(None);

        // when
        let (_, outputs) = get_transaction_inputs_outputs(&[call], Default::default(), &wallet);

        // then
        let expected_outputs = (0..=1)
            .map(|i| Output::contract(i, Bytes32::zeroed(), Bytes32::zeroed()))
            .collect::<Vec<_>>();

        assert_eq!(outputs, expected_outputs);
    }

    #[test]
    fn change_per_asset_id_added() {
        // given
        let asset_ids = [AssetId::default(), AssetId::from([1; 32])];

        let coins = asset_ids
            .into_iter()
            .map(|asset_id| {
                let coin = CoinType::Coin(Coin {
                    amount: 100,
                    block_created: 0u32,
                    asset_id,
                    utxo_id: Default::default(),
                    maturity: 0u32,
                    owner: Default::default(),
                    status: CoinStatus::Unspent,
                });
                Input::resource_signed(coin)
            })
            .collect();
        let call = ContractCall::new_with_random_id();

        let wallet = WalletUnlocked::new_random(None);

        // when
        let (_, outputs) = get_transaction_inputs_outputs(&[call], coins, &wallet);

        // then
        let change_outputs: HashSet<Output> = outputs[1..].iter().cloned().collect();

        let expected_change_outputs = asset_ids
            .into_iter()
            .map(|asset_id| Output::Change {
                to: wallet.address().into(),
                amount: 0,
                asset_id,
            })
            .collect();

        assert_eq!(change_outputs, expected_change_outputs);
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

        let wallet = WalletUnlocked::new_random(None);

        // when
        let (_, outputs) = get_transaction_inputs_outputs(&calls, Default::default(), &wallet);

        // then
        let actual_variable_outputs: HashSet<Output> = outputs[2..].iter().cloned().collect();
        let expected_outputs: HashSet<Output> = variable_outputs.into();

        assert_eq!(expected_outputs, actual_variable_outputs);
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
