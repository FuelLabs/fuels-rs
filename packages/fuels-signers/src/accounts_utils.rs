use fuel_tx::{ConsensusParameters, Output, Receipt};
use fuel_types::MessageId;
use fuels_types::bech32::Bech32Address;
use fuels_types::constants::BASE_ASSET_ID;
use fuels_types::input::Input;
use fuels_types::transaction_builders::TransactionBuilder;

pub fn extract_message_id(receipts: &[Receipt]) -> Option<&MessageId> {
    receipts
        .iter()
        .find(|r| matches!(r, Receipt::MessageOut { .. }))
        .and_then(|m| m.message_id())
}

pub fn calculate_base_amount_with_fee<Tx, Tb: TransactionBuilder<Tx>>(
    tb: &Tb,
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
pub fn adjust_inputs<Tx, Tb: TransactionBuilder<Tx>>(tb: &mut Tb, new_base_inputs: Vec<Input>) {
    let (_, remaining_inputs): (Vec<_>, Vec<_>) = tb.inputs().iter().cloned().partition(|input| {
        matches!(input , Input::ResourceSigned { resource , .. } if resource.asset_id() == BASE_ASSET_ID) ||
            matches!(input , Input::ResourcePredicate { resource, .. } if resource.asset_id() == BASE_ASSET_ID)
    });

    let adjusted_inputs: ::std::vec::Vec<_> = remaining_inputs
        .into_iter()
        .chain(new_base_inputs.into_iter())
        .collect();

    *tb.inputs_mut() = adjusted_inputs
}

pub fn adjust_outputs<Tx, Tb: TransactionBuilder<Tx>>(
    tb: &mut Tb,
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
