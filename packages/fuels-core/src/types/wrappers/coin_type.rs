#![cfg(feature = "std")]

use fuel_core_client::client::types::CoinType as ClientCoinType;

use crate::types::{Address, AssetId, coin::Coin, coin_type_id::CoinTypeId, message::Message};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CoinType {
    Coin(Coin),
    Message(Message),
    Unknown,
}

impl From<ClientCoinType> for CoinType {
    fn from(client_resource: ClientCoinType) -> Self {
        match client_resource {
            ClientCoinType::Coin(coin) => CoinType::Coin(coin.into()),
            ClientCoinType::MessageCoin(message) => CoinType::Message(message.into()),
            ClientCoinType::Unknown => CoinType::Unknown,
        }
    }
}

impl CoinType {
    pub fn id(&self) -> Option<CoinTypeId> {
        match self {
            CoinType::Coin(coin) => Some(CoinTypeId::UtxoId(coin.utxo_id)),
            CoinType::Message(message) => Some(CoinTypeId::Nonce(message.nonce)),
            CoinType::Unknown => None,
        }
    }

    pub fn amount(&self) -> u64 {
        match self {
            CoinType::Coin(coin) => coin.amount,
            CoinType::Message(message) => message.amount,
            CoinType::Unknown => 0,
        }
    }

    pub fn coin_asset_id(&self) -> Option<AssetId> {
        match self {
            CoinType::Coin(coin) => Some(coin.asset_id),
            CoinType::Message(_) => None,
            CoinType::Unknown => None,
        }
    }

    pub fn asset_id(&self, base_asset_id: AssetId) -> Option<AssetId> {
        match self {
            CoinType::Coin(coin) => Some(coin.asset_id),
            CoinType::Message(_) => Some(base_asset_id),
            CoinType::Unknown => None,
        }
    }

    pub fn owner(&self) -> Option<&Address> {
        match self {
            CoinType::Coin(coin) => Some(&coin.owner),
            CoinType::Message(message) => Some(&message.recipient),
            CoinType::Unknown => None,
        }
    }
}
