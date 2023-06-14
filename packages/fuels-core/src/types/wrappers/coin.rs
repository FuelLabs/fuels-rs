#![cfg(feature = "std")]

use fuel_core_chain_config::CoinConfig;
use fuel_core_client::client::schema::coins::Coin as ClientCoin;
use fuel_tx::{AssetId, UtxoId};

use crate::types::bech32::Bech32Address;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum CoinStatus {
    #[default]
    Unspent,
    Spent,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct Coin {
    pub amount: u64,
    pub block_created: u32,
    pub asset_id: AssetId,
    pub utxo_id: UtxoId,
    pub maturity: u32,
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
            status: CoinStatus::Unspent,
        }
    }
}

impl From<Coin> for CoinConfig {
    fn from(coin: Coin) -> CoinConfig {
        Self {
            tx_id: Some(*coin.utxo_id.tx_id()),
            output_index: Some(coin.utxo_id.output_index()),
            tx_pointer_block_height: Some(coin.block_created.into()),
            tx_pointer_tx_idx: None,
            maturity: Some(coin.maturity.into()),
            owner: coin.owner.into(),
            amount: coin.amount,
            asset_id: coin.asset_id,
        }
    }
}
