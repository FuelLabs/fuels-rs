#![cfg(feature = "std")]

use fuel_core_chain_config::MessageConfig;
use fuel_core_client::client::types::{
    coins::MessageCoin as ClientMessageCoin, message::Message as ClientMessage,
};
use fuel_tx::{Input, MessageId};
use fuel_types::Nonce;

use crate::types::bech32::Bech32Address;

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
    pub nonce: Nonce,
    pub data: Vec<u8>,
    pub da_height: u64,
    pub status: MessageStatus,
}

impl Message {
    pub fn message_id(&self) -> MessageId {
        Input::compute_message_id(
            &(&self.sender).into(),
            &(&self.recipient).into(),
            &self.nonce,
            self.amount,
            &self.data,
        )
    }
}

impl From<ClientMessage> for Message {
    fn from(message: ClientMessage) -> Self {
        Self {
            amount: message.amount,
            sender: message.sender.into(),
            recipient: message.recipient.into(),
            nonce: message.nonce,
            data: message.data,
            da_height: message.da_height,
            status: MessageStatus::Unspent,
        }
    }
}

impl From<ClientMessageCoin> for Message {
    fn from(message: ClientMessageCoin) -> Self {
        Self {
            amount: message.amount,
            sender: message.sender.into(),
            recipient: message.recipient.into(),
            nonce: message.nonce,
            data: Default::default(),
            da_height: message.da_height,
            status: MessageStatus::Unspent,
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
