use fuel_tx::{Input, Output, Receipt, TxPointer, UtxoId};
use fuels_types::bech32::Bech32Address;
use std::collections::HashSet;

use fuel_gql_client::prelude::PanicReason;
use fuels_core::constants::{BASE_ASSET_ID, FAILED_TRANSFER_TO_ADDRESS_SIGNAL};
use fuels_core::tx::{AssetId, Bytes32, ContractId};
use fuels_types::resource::Resource;
use itertools::Itertools;

pub(crate) fn is_missing_output_variables(receipts: &[Receipt]) -> bool {
    receipts.iter().any(
        |r| matches!(r, Receipt::Revert { ra, .. } if *ra == FAILED_TRANSFER_TO_ADDRESS_SIGNAL),
    )
}

pub(crate) fn find_contract_not_in_inputs(receipts: &[Receipt]) -> Option<&Receipt> {
    receipts.iter().find(
            |r| matches!(r, Receipt::Panic { reason, .. } if *reason.reason() == PanicReason::ContractNotInInputs ),
        )
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

/// Sum up the amounts required in each call for each asset ID, so you can get a total for each
/// asset over all calls.
pub(crate) fn sum_up_amounts_for_each_asset_id(
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

pub(crate) fn extract_unique_asset_ids(spendable_coins: &[Resource]) -> HashSet<AssetId> {
    spendable_coins
        .iter()
        .map(|resource| match resource {
            Resource::Coin(coin) => coin.asset_id,
            Resource::Message(_) => BASE_ASSET_ID,
        })
        .collect()
}

pub(crate) fn generate_asset_change_outputs(
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

pub(crate) fn convert_to_signed_resources(spendable_resources: Vec<Resource>) -> Vec<Input> {
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
