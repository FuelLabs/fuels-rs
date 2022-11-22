use fuel_gql_client::client::schema::coin::{Coin as SchemaCoin, CoinStatus as SchemaCoinStatus};
use fuel_tx::{Address, AssetId, UtxoId};

pub enum CoinStatus {
    Unspent,
    Spent,
}

impl From<SchemaCoinStatus> for CoinStatus {
    fn from(schema_coin_status: SchemaCoinStatus) -> Self {
        match schema_coin_status {
            SchemaCoinStatus::Unspent => CoinStatus::Unspent,
            SchemaCoinStatus::Spent => CoinStatus::Spent,
        }
    }
}

pub struct Coin {
    schema_coin: SchemaCoin,
}

impl From<SchemaCoin> for Coin {
    fn from(schema_coin: SchemaCoin) -> Self {
        Self { schema_coin }
    }
}

impl Coin {
    pub fn amount(&self) -> u64 {
        self.schema_coin.amount.0
    }

    pub fn block_created(&self) -> u64 {
        self.schema_coin.block_created.0
    }

    pub fn asset_id(&self) -> AssetId {
        AssetId::from(self.schema_coin.asset_id.0 .0)
    }

    pub fn utxo_id(&self) -> UtxoId {
        self.schema_coin.utxo_id.0 .0
    }

    pub fn maturity(&self) -> u64 {
        self.schema_coin.maturity.0
    }

    pub fn owner(&self) -> Address {
        Address::from(self.schema_coin.owner.0 .0)
    }

    pub fn status(&self) -> CoinStatus {
        self.schema_coin.status.into()
    }
}
