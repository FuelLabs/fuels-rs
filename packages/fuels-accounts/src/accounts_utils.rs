use fuel_tx::{AssetId, Output, Receipt, UtxoId};
use fuel_types::Nonce;
use fuels_core::types::{
    bech32::Bech32Address,
    coin::Coin,
    coin_type::CoinType,
    coin_type_id::CoinTypeId,
    errors::{error, Error, Result},
    input::Input,
    transaction_builders::TransactionBuilder,
};
use itertools::{Either, Itertools};

use crate::provider::Provider;

pub fn extract_message_nonce(receipts: &[Receipt]) -> Option<Nonce> {
    receipts.iter().find_map(|m| m.nonce()).copied()
}

pub async fn calculate_missing_base_amount(
    tb: &impl TransactionBuilder,
    available_base_amount: u64,
    reserved_base_amount: u64,
    provider: &Provider,
) -> Result<u64> {
    let max_fee = tb.estimate_max_fee(provider).await?;

    let total_used = max_fee + reserved_base_amount;
    let missing_amount = if total_used > available_base_amount {
        total_used - available_base_amount
    } else if !is_consuming_utxos(tb) {
        // A tx needs to have at least 1 spendable input
        // Enforce a minimum required amount on the base asset if no other inputs are present
        1
    } else {
        0
    };

    Ok(missing_amount)
}

pub fn available_base_assets_and_amount(
    tb: &impl TransactionBuilder,
    base_asset_id: &AssetId,
) -> (Vec<CoinTypeId>, u64) {
    let mut sum = 0;
    let iter =
        tb.inputs()
            .iter()
            .filter_map(|input| match input {
                Input::ResourceSigned { resource, .. }
                | Input::ResourcePredicate { resource, .. } => match resource {
                    CoinType::Coin(Coin {
                        amount, asset_id, ..
                    }) if asset_id == base_asset_id => {
                        sum += amount;
                        Some(resource.id())
                    }
                    CoinType::Message(message) => {
                        sum += message.amount;
                        Some(resource.id())
                    }
                    _ => None,
                },
                _ => None,
            })
            .collect_vec();

    (iter, sum)
}

pub fn split_into_utxo_ids_and_nonces(
    excluded_coins: Option<Vec<CoinTypeId>>,
) -> (Vec<UtxoId>, Vec<Nonce>) {
    excluded_coins
        .map(|excluded_coins| {
            excluded_coins
                .iter()
                .partition_map(|coin_id| match coin_id {
                    CoinTypeId::UtxoId(utxo_id) => Either::Left(*utxo_id),
                    CoinTypeId::Nonce(nonce) => Either::Right(*nonce),
                })
        })
        .unwrap_or_default()
}

fn is_consuming_utxos(tb: &impl TransactionBuilder) -> bool {
    tb.inputs()
        .iter()
        .any(|input| !matches!(input, Input::Contract { .. }))
}

pub fn add_base_change_if_needed(
    tb: &mut impl TransactionBuilder,
    address: &Bech32Address,
    base_asset_id: &AssetId,
) {
    let is_base_change_present = tb.outputs().iter().any(|output| {
        matches!(output , Output::Change { asset_id , .. }
                                        if asset_id == base_asset_id)
    });

    if !is_base_change_present {
        tb.outputs_mut()
            .push(Output::change(address.into(), 0, *base_asset_id));
    }
}

pub(crate) fn try_provider_error() -> Error {
    error!(
        Other,
        "no provider available. Make sure to use `set_provider`"
    )
}
