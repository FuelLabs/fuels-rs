use fuel_tx::{Output, Receipt};
use fuel_types::Nonce;
use fuels_core::{
    constants::BASE_ASSET_ID,
    types::{
        bech32::Bech32Address,
        errors::{error, Result},
        input::Input,
        transaction_builders::TransactionBuilder,
    },
};

use crate::provider::Provider;

pub fn extract_message_nonce(receipts: &[Receipt]) -> Option<Nonce> {
    receipts.iter().find_map(|m| m.nonce()).copied()
}

pub async fn calculate_missing_base_amount(
    tb: &impl TransactionBuilder,
    used_base_amount: u64,
    provider: &Provider,
) -> Result<u64> {
    let transaction_fee = tb
        .fee_checked_from_tx(provider)
        .await?
        .ok_or(error!(InvalidData, "Error calculating TransactionFee"))?;

    let available_amount = available_base_amount(tb);

    let total_used = transaction_fee
        .max_fee()
        .checked_add(used_base_amount)
        .ok_or_else(|| {
            error!(
                InvalidType,
                "Addition overflow while calculating total_used for max_fee {:?} and used_base_amount {used_base_amount:?}",
                transaction_fee.max_fee()
            )
        })?;

    let missing_amount = if total_used > available_amount {
        total_used.checked_sub(available_amount)
        .ok_or_else(|| {
            error!(
                InvalidType,
                "Subtraction overflow while calculating missing_amount for available_amount {available_amount:?}"
            )
        })?
    } else if !is_consuming_utxos(tb) {
        // A tx needs to have at least 1 spendable input
        // Enforce a minimum required amount on the base asset if no other inputs are present
        1
    } else {
        0
    };

    Ok(missing_amount)
}

fn available_base_amount(tb: &impl TransactionBuilder) -> u64 {
    tb.inputs()
        .iter()
        .filter_map(|input| match (input.amount(), input.asset_id()) {
            (Some(amount), Some(asset_id)) if asset_id == BASE_ASSET_ID => Some(amount),
            _ => None,
        })
        .sum()
}

fn is_consuming_utxos(tb: &impl TransactionBuilder) -> bool {
    tb.inputs()
        .iter()
        .any(|input| !matches!(input, Input::Contract { .. }))
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
