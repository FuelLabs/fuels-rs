use fuel_core_chain_config::{CoinConfig, MessageConfig};
use fuels_core::types::{coin::Coin, message::Message};

pub(crate) fn into_coin_configs(coins: Vec<Coin>) -> Vec<CoinConfig> {
    coins
        .into_iter()
        .map(Into::into)
        .collect::<Vec<CoinConfig>>()
}

pub(crate) fn into_message_configs(messages: Vec<Message>) -> Vec<MessageConfig> {
    messages
        .into_iter()
        .map(Into::into)
        .collect::<Vec<MessageConfig>>()
}
