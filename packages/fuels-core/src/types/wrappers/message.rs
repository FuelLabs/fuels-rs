#![cfg(feature = "std")]

use fuel_core_chain_config::MessageConfig;
use fuel_core_client::client::schema::coins::MessageCoin as ClientMessageCoin;
use fuel_core_client::client::schema::message::Message as ClientMessage;
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
            amount: message.amount.0,
            sender: message.sender.0 .0.into(),
            recipient: message.recipient.0 .0.into(),
            nonce: message.nonce.into(),
            data: message.data.0 .0,
            da_height: message.da_height.0,
            status: MessageStatus::Unspent,
        }
    }
}

impl From<ClientMessageCoin> for Message {
    fn from(message: ClientMessageCoin) -> Self {
        Self {
            amount: message.amount.0,
            sender: message.sender.0 .0.into(),
            recipient: message.recipient.0 .0.into(),
            nonce: message.nonce.into(),
            data: Default::default(),
            da_height: message.da_height.0,
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
