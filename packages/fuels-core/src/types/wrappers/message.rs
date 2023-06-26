#![cfg(feature = "std")]

use fuel_core_chain_config::MessageConfig;
use fuel_core_client::client::types::{
    coins::MessageCoin as ClientMessageCoin,
    message::Message as ClientMessage,
    primitives::{Address, Nonce},
};
use fuel_tx::{Input, MessageId};

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
            &self.sender.clone().into(),
            &self.recipient.clone().into(),
            &Nonce::from(*self.nonce),
            self.amount,
            &self.data,
        )
    }
}

impl From<ClientMessage> for Message {
    fn from(message: ClientMessage) -> Self {
        let sender: Address = message.sender;
        let recipient: Address = message.recipient;
        Self {
            amount: message.amount,
            sender: Bech32Address::from(sender),
            recipient: Bech32Address::from(recipient),
            nonce: message.nonce,
            data: message.data,
            da_height: message.da_height,
            status: MessageStatus::Unspent,
        }
    }
}

impl From<ClientMessageCoin> for Message {
    fn from(message: ClientMessageCoin) -> Self {
        let sender: Address = message.sender;
        let recipient: Address = message.recipient;
        Self {
            amount: message.amount,
            sender: Bech32Address::from(sender),
            recipient: Bech32Address::from(recipient),
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
