#![cfg(feature = "std")]

use fuel_core_chain_config::MessageConfig;
use fuel_core_client::client::schema::message::{
    Message as ClientMessage, MessageStatus as ClientMessageStatus,
};
use fuel_tx::{Input, MessageId};

use crate::bech32::Bech32Address;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum MessageStatus {
    #[default]
    Unspent,
    Spent,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Message {
    pub amount: u64,
    pub sender: Bech32Address,
    pub recipient: Bech32Address,
    pub nonce: u64,
    pub data: Vec<u8>,
    pub da_height: u64,
    pub status: MessageStatus,
}

impl Message {
    pub fn message_id(&self) -> MessageId {
        Input::compute_message_id(
            &(&self.sender).into(),
            &(&self.recipient).into(),
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
            sender: message.sender.0 .0.into(),
            recipient: message.recipient.0 .0.into(),
            nonce: message.nonce.0,
            data: message.data.0 .0,
            da_height: message.da_height.0,
            status: MessageStatus::from(message.status),
        }
    }
}

impl From<ClientMessageStatus> for MessageStatus {
    fn from(status: ClientMessageStatus) -> Self {
        match status {
            ClientMessageStatus::Unspent => MessageStatus::Unspent,
            ClientMessageStatus::Spent => MessageStatus::Spent,
        }
    }
}

impl From<Message> for MessageConfig {
    fn from(message: Message) -> MessageConfig {
        MessageConfig {
            sender: message.sender.into(),
            recipient: message.recipient.into(),
            nonce: message.nonce,
            amount: message.amount,
            data: message.data,
            da_height: message.da_height.into(),
        }
    }
}
