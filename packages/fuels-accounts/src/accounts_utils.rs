use fuel_tx::{ConsensusParameters, Output, Receipt};
use fuel_types::MessageId;
use fuels_core::{
    constants::BASE_ASSET_ID,
    types::{bech32::Bech32Address, input::Input, transaction_builders::TransactionBuilder},
};

pub fn extract_message_id(receipts: &[Receipt]) -> Option<MessageId> {
    receipts.iter().find_map(|m| m.message_id())
}

pub fn calculate_base_amount_with_fee(
    tb: &impl TransactionBuilder,
    consensus_params: &ConsensusParameters,
    previous_base_amount: u64,
) -> u64 {
    let transaction_fee = tb
        .fee_checked_from_tx(consensus_params)
        .expect("Error calculating TransactionFee");

    let mut new_base_amount = transaction_fee.total() + previous_base_amount;

    // If the tx doesn't consume any UTXOs, attempting to repeat it will lead to an
    // error due to non unique tx ids (e.g. repeated contract call with configured gas cost of 0).
    // Here we enforce a minimum amount on the base asset to avoid this
    let is_consuming_utxos = tb
        .inputs()
        .iter()
        .any(|input| !matches!(input, Input::Contract { .. }));
    const MIN_AMOUNT: u64 = 1;
    if !is_consuming_utxos && new_base_amount == 0 {
        new_base_amount = MIN_AMOUNT;
    }
    new_base_amount
}

pub fn adjust_inputs(
    tb: &mut impl TransactionBuilder,
    new_base_inputs: impl IntoIterator<Item = Input>,
) {
    let adjusted_inputs = tb
        .inputs()
        .iter()
        .filter(|input| {
            !matches!(input , Input::ResourceSigned { resource , .. }
                | Input::ResourcePredicate { resource, .. } if resource.asset_id() == BASE_ASSET_ID)
        })
        .cloned()
        .chain(new_base_inputs)
        .collect();

    *tb.inputs_mut() = adjusted_inputs
}

pub fn adjust_outputs(
    tb: &mut impl TransactionBuilder,
    address: &Bech32Address,
    new_base_amount: u64,
) {
    let is_base_change_present = tb.outputs().iter().any(|output| {
        matches!(output , Output::Change { asset_id , .. }
                                        if asset_id == & BASE_ASSET_ID)
    });

    if !is_base_change_present && new_base_amount != 0 {
        tb.outputs_mut()
            .push(Output::change(address.into(), 0, BASE_ASSET_ID));
    }
}
