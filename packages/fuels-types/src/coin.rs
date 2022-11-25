use fuel_gql_client::client::schema::coin::{Coin as ClientCoin, CoinStatus as ClientCoinStatus};
use fuel_tx::{Address, AssetId, UtxoId};

#[derive(Debug)]

pub enum CoinStatus {
    Unspent,
    Spent,
}

impl From<ClientCoinStatus> for CoinStatus {
    fn from(client_coin_status: ClientCoinStatus) -> Self {
        match client_coin_status {
            ClientCoinStatus::Unspent => CoinStatus::Unspent,
            ClientCoinStatus::Spent => CoinStatus::Spent,
        }
    }
}

#[derive(Debug)]
pub struct Coin {
    pub amount: u64,
    pub block_created: u64,
    pub asset_id: AssetId,
    pub utxo_id: UtxoId,
    pub maturity: u64,
    pub owner: Address,
    pub status: CoinStatus,
}

impl From<ClientCoin> for Coin {
    fn from(client_coin: ClientCoin) -> Self {
        Self {
            amount: client_coin.amount.0,
            block_created: client_coin.block_created.0,
            asset_id: client_coin.asset_id.0 .0,
            utxo_id: client_coin.utxo_id.0 .0,
            maturity: client_coin.maturity.0,
            owner: client_coin.owner.0 .0,
            status: client_coin.status.into(),
        }
    }
}
