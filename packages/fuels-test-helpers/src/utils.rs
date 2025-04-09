use fuel_core_chain_config::{CoinConfig, MessageConfig};
use fuels_core::types::{
    coin::{Coin, DataCoin},
    message::Message,
};

pub(crate) fn into_coin_configs(
    coins: Vec<Coin>, /*, data_coins: Vec<DataCoin>*/
) -> Vec<CoinConfig> {
    coins
        .into_iter()
        .map(Into::into)
        // .chain(data_coins.into_iter().map(Into::into))
        .collect::<Vec<CoinConfig>>()
}

pub(crate) fn into_coin_configs2(coins: Vec<Coin>, data_coins: Vec<DataCoin>) -> Vec<CoinConfig> {
    coins
        .into_iter()
        .map(Into::into)
        .chain(data_coins.into_iter().map(Into::into))
        .collect::<Vec<CoinConfig>>()
}

pub(crate) fn into_message_configs(messages: Vec<Message>) -> Vec<MessageConfig> {
    messages
        .into_iter()
        .map(Into::into)
        .collect::<Vec<MessageConfig>>()
}
