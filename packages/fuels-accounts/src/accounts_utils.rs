use fuel_tx::{Output, Receipt};
use fuel_types::MessageId;
use fuels_core::{
    constants::BASE_ASSET_ID,
    types::{
        bech32::Bech32Address,
        errors::{error, Error, Result},
        input::Input,
        transaction::Transaction,
        transaction_builders::TransactionBuilder,
    },
};

use crate::Account;

pub fn extract_message_id(receipts: &[Receipt]) -> Option<MessageId> {
    receipts.iter().find_map(|m| m.message_id())
}

pub fn calculate_missing_base_amount(
    account: &impl Account,
    tb: &impl TransactionBuilder,
) -> Result<u64> {
    let consensus_parameters = tb.consensus_parameters();
    let tx = account.finalize_tx(tb.clone())?;
    let transaction_fee = tx
        .fee_checked_from_tx(&consensus_parameters)
        .ok_or(error!(InvalidData, "Error calculating TransactionFee"))?;

    let input_base_amount: u64 = tb
        .inputs()
        .iter()
        .filter_map(|input| match (input.amount(), input.asset_id()) {
            (Some(amount), Some(asset_id)) if asset_id == BASE_ASSET_ID => Some(amount),
            _ => None,
        })
        .sum();

    let output_base_amount: u64 = tb
        .outputs()
        .iter()
        .filter_map(|output| match (output.amount(), output.asset_id()) {
            (Some(amount), Some(asset_id)) if *asset_id == BASE_ASSET_ID => Some(amount),
            _ => None,
        })
        .sum();

    let needed_base_amount = transaction_fee.max_fee() + output_base_amount;
    let missing_amount = if needed_base_amount > input_base_amount {
        input_base_amount.abs_diff(needed_base_amount)
    } else {
        // A tx needs to have at least 1 spendable input
        // We enforce a minimum amount on the base asset if no other inputs are present
        let is_consuming_utxos = tb
            .inputs()
            .iter()
            .any(|input| !matches!(input, Input::Contract { .. }));
        if !is_consuming_utxos {
            1
        } else {
            0
        }
    };

    Ok(missing_amount)
}

pub fn adjust_inputs_outputs(
    tb: &mut impl TransactionBuilder,
    new_base_inputs: impl IntoIterator<Item = Input>,
    address: &Bech32Address,
) {
    tb.inputs_mut().extend(new_base_inputs);

    let is_base_change_present = tb.outputs().iter().any(|output| {
        matches!(output , Output::Change { asset_id , .. }
                                        if asset_id == & BASE_ASSET_ID)
    });

    if !is_base_change_present {
        tb.outputs_mut()
            .push(Output::change(address.into(), 0, BASE_ASSET_ID));
    }
}
