use fuel_gql_client::client::schema::message::Message as ClientMessage;
use fuel_tx::{Address, MessageId};

#[derive(Debug)]
pub struct Message {
    pub message_id: MessageId,
    pub amount: u64,
    pub sender: Address,
    pub recipient: Address,
    pub nonce: u64,
    pub data: Vec<u8>,
    pub da_height: u64,
    pub fuel_block_spend: Option<u64>,
}

impl From<ClientMessage> for Message {
    fn from(client_message: ClientMessage) -> Self {
        Self {
            message_id: client_message.message_id.0 .0,
            amount: client_message.amount.0,
            sender: client_message.sender.0 .0,
            recipient: client_message.recipient.0 .0,
            nonce: client_message.nonce.0,
            data: client_message.data.0 .0,
            da_height: client_message.da_height.0,
            fuel_block_spend: client_message.fuel_block_spend.map(|bs| bs.0),
        }
    }
}
