#![cfg(feature = "std")]

use fuel_core_client::client::types::CoinType as ClientCoinType;
use fuel_types::AssetId;

use crate::{
    constants::BASE_ASSET_ID,
    types::{bech32::Bech32Address, coin::Coin, message::Message},
};

use super::coin_type_id::CoinTypeId;

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
    pub fn id(&self) -> CoinTypeId {
        match self {
            CoinType::Coin(coin) => CoinTypeId::UtxoId(coin.utxo_id),
            CoinType::Message(message) => CoinTypeId::Nonce(message.nonce),
        }
    }

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

    pub fn owner(&self) -> &Bech32Address {
        match self {
            CoinType::Coin(coin) => &coin.owner,
            CoinType::Message(message) => &message.recipient,
        }
    }
}
