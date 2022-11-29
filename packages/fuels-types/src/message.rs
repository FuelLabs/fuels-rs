use fuel_chain_config::MessageConfig;
#[cfg(feature = "fuel-core-lib")]
use fuel_core::model::Message as ClientMessage;

#[cfg(not(feature = "fuel-core-lib"))]
use fuel_gql_client::client::schema::message::Message as ClientMessage;

use fuel_tx::{Address, Input, MessageId};

#[derive(Debug, Clone)]
pub struct Message {
    pub amount: u64,
    pub sender: Address,
    pub recipient: Address,
    pub nonce: u64,
    pub data: Vec<u8>,
    pub da_height: u64,
    pub fuel_block_spend: Option<u64>,
}

impl Message {
    pub fn message_id(&self) -> MessageId {
        Input::compute_message_id(
            &self.sender,
            &self.recipient,
            self.nonce,
            self.amount,
            &self.data,
        )
    }
}

impl From<ClientMessage> for Message {
    fn from(message: ClientMessage) -> Self {
        Self {
            amount: message.amount.0,
            sender: message.sender.0 .0,
            recipient: message.recipient.0 .0,
            nonce: message.nonce.0,
            data: message.data.0 .0,
            da_height: message.da_height.0,
            fuel_block_spend: message.fuel_block_spend.map(|bs| bs.0),
        }
    }
}

impl From<Message> for MessageConfig {
    fn from(message: Message) -> MessageConfig {
        MessageConfig {
            sender: message.sender,
            recipient: message.recipient,
            nonce: message.nonce,
            amount: message.amount,
            data: message.data,
            da_height: message.da_height.into(),
        }
    }
}
