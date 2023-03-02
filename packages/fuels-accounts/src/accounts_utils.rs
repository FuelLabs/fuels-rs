use fuel_tx::{Input as FuelInput, Receipt, TxPointer};
use fuel_types::{AssetId, MessageId};
use fuels_types::coin::Coin;
use fuels_types::input::Input;
use fuels_types::message::Message;

pub fn extract_message_id(receipts: &[Receipt]) -> Option<&MessageId> {
    receipts
        .iter()
        .find(|r| matches!(r, Receipt::MessageOut { .. }))
        .and_then(|m| m.message_id())
}
