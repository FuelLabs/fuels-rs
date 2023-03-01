use fuel_tx::{Input as FuelInput, Receipt, TxPointer};
use fuel_types::{AssetId, MessageId};
use fuels_types::coin::Coin;
use fuels_types::input::Input;
use fuels_types::message::Message;

pub fn create_coin_input(coin: Coin, asset_id: AssetId, witness_index: u8) -> FuelInput {
    FuelInput::coin_signed(
        coin.utxo_id,
        coin.owner.into(),
        coin.amount,
        asset_id,
        TxPointer::default(),
        witness_index,
        0,
    )
}

pub fn create_message_input(message: Message, witness_index: u8) -> FuelInput {
    FuelInput::message_signed(
        message.message_id(),
        message.sender.into(),
        message.recipient.into(),
        message.amount,
        message.nonce,
        witness_index,
        message.data,
    )
}

pub fn create_coin_predicate(
    coin: Coin,
    asset_id: AssetId,
    code: Vec<u8>,
    predicate_data: Vec<u8>,
) -> FuelInput {
    FuelInput::coin_predicate(
        coin.utxo_id,
        coin.owner.into(),
        coin.amount,
        asset_id,
        TxPointer::new(0, 0),
        0,
        code,
        predicate_data,
    )
}

pub fn create_message_predicate(
    message: Message,
    code: Vec<u8>,
    predicate_data: Vec<u8>,
) -> FuelInput {
    FuelInput::message_predicate(
        message.message_id(),
        message.sender.into(),
        message.recipient.into(),
        message.amount,
        message.nonce,
        message.data,
        code,
        predicate_data,
    )
}

pub fn extract_message_id(receipts: &[Receipt]) -> Option<&MessageId> {
    receipts
        .iter()
        .find(|r| matches!(r, Receipt::MessageOut { .. }))
        .and_then(|m| m.message_id())
}
