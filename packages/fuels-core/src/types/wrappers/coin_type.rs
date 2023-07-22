#![cfg(feature = "std")]

use fuel_core_client::client::types::CoinType as ClientCoinType;
use fuel_types::AssetId;

use crate::{
    constants::BASE_ASSET_ID,
    types::{coin::Coin, message::Message},
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CoinType {
    Coin(Coin),
    Message(Message),
}

impl TryFrom<ClientCoinType> for CoinType {
    type Error = std::io::Error;

    fn try_from(client_resource: ClientCoinType) -> Result<Self, Self::Error> {
        match client_resource {
            ClientCoinType::Coin(coin) => Ok(CoinType::Coin(coin.into())),
            ClientCoinType::MessageCoin(message) => Ok(CoinType::Message(message.into())),
            ClientCoinType::Unknown => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Got unknown `ClientCoinType`",
            )),
        }
    }
}

impl CoinType {
    pub fn amount(&self) -> u64 {
        match self {
            CoinType::Coin(coin) => coin.amount,
            CoinType::Message(message) => message.amount,
        }
    }

    pub fn asset_id(&self) -> AssetId {
        match self {
            CoinType::Coin(coin) => coin.asset_id,
            CoinType::Message(_) => BASE_ASSET_ID,
        }
    }
}
