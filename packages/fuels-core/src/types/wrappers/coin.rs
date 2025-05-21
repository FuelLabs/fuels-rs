#![cfg(feature = "std")]

use fuel_core_chain_config::CoinConfig;
use fuel_core_client::client::types::{
    coins::Coin as ClientCoin,
    primitives::{AssetId, UtxoId},
};

use crate::types::Address;

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct Coin {
    pub amount: u64,
    pub asset_id: AssetId,
    pub utxo_id: UtxoId,
    pub owner: Address,
}

impl From<ClientCoin> for Coin {
    fn from(coin: ClientCoin) -> Self {
        Self {
            amount: coin.amount,
            asset_id: coin.asset_id,
            utxo_id: coin.utxo_id,
            owner: coin.owner,
        }
    }
}

impl From<Coin> for CoinConfig {
    fn from(coin: Coin) -> CoinConfig {
        Self {
            tx_id: *coin.utxo_id.tx_id(),
            output_index: coin.utxo_id.output_index(),
            owner: coin.owner,
            amount: coin.amount,
            asset_id: coin.asset_id,
            ..Default::default()
        }
    }
}
