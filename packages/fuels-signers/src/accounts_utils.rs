use fuel_tx::Receipt;
use fuel_types::MessageId;

pub fn extract_message_id(receipts: &[Receipt]) -> Option<&MessageId> {
    receipts
        .iter()
        .find(|r| matches!(r, Receipt::MessageOut { .. }))
        .and_then(|m| m.message_id())
}
