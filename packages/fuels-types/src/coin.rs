#[cfg(feature = "fuel-core-lib")]
use fuel_core::model::{Coin as ClientCoin, CoinStatus as ClientCoinStatus};

#[cfg(not(feature = "fuel-core-lib"))]
use fuel_gql_client::client::schema::coin::{Coin as ClientCoin, CoinStatus as ClientCoinStatus};

use fuel_chain_config::CoinConfig;
use fuel_tx::{AssetId, UtxoId};

use crate::bech32::Bech32Address;

#[derive(Debug, Clone)]

pub enum CoinStatus {
    Unspent,
    Spent,
}

impl From<ClientCoinStatus> for CoinStatus {
    fn from(coin_status: ClientCoinStatus) -> Self {
        match coin_status {
            ClientCoinStatus::Unspent => CoinStatus::Unspent,
            ClientCoinStatus::Spent => CoinStatus::Spent,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Coin {
    pub amount: u64,
    pub block_created: u64,
    pub asset_id: AssetId,
    pub utxo_id: UtxoId,
    pub maturity: u64,
    pub owner: Bech32Address,
    pub status: CoinStatus,
}

impl From<ClientCoin> for Coin {
    fn from(coin: ClientCoin) -> Self {
        Self {
            amount: coin.amount.0,
            block_created: coin.block_created.0,
            asset_id: coin.asset_id.0 .0,
            utxo_id: coin.utxo_id.0 .0,
            maturity: coin.maturity.0,
            owner: coin.owner.0 .0.into(),
            status: coin.status.into(),
        }
    }
}

impl From<Coin> for CoinConfig {
    fn from(coin: Coin) -> CoinConfig {
        Self {
            tx_id: Some(*coin.utxo_id.tx_id()),
            output_index: Some(coin.utxo_id.output_index() as u64),
            block_created: Some(coin.block_created.into()),
            maturity: Some(coin.maturity.into()),
            owner: coin.owner.into(),
            amount: coin.amount,
            asset_id: coin.asset_id,
        }
    }
}
