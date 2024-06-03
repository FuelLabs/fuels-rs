use std::{
    net::{Ipv4Addr, SocketAddr},
    path::PathBuf,
    time::Duration,
};

pub use fuel_core_chain_config::{ChainConfig, StateConfig};

pub(crate) const MAX_DATABASE_CACHE_SIZE: usize = 10 * 1024 * 1024;

#[derive(Clone, Debug)]
pub enum Trigger {
    Instant,
    Never,
    Interval { block_time: Duration },
}

#[cfg(feature = "fuel-core-lib")]
impl From<Trigger> for fuel_core_poa::Trigger {
    fn from(value: Trigger) -> Self {
        match value {
            Trigger::Instant => fuel_core_poa::Trigger::Instant,
            Trigger::Never => fuel_core_poa::Trigger::Never,
            Trigger::Interval { block_time } => fuel_core_poa::Trigger::Interval { block_time },
        }
    }
}

#[derive(Clone, Debug)]
pub enum DbType {
    InMemory,
    RocksDb(Option<PathBuf>),
}

#[cfg(feature = "fuel-core-lib")]
impl From<DbType> for fuel_core::service::DbType {
    fn from(value: DbType) -> Self {
        match value {
            DbType::InMemory => fuel_core::service::DbType::InMemory,
            DbType::RocksDb(..) => fuel_core::service::DbType::RocksDb,
        }
    }
}

#[derive(Clone, Debug)]
pub struct NodeConfig {
    pub addr: SocketAddr,
    pub max_database_cache_size: Option<usize>,
    pub database_type: DbType,
    pub utxo_validation: bool,
    pub debug: bool,
    pub block_production: Trigger,
    pub vm_backtrace: bool,
    pub silent: bool,
    pub static_gas_price: u64,
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            addr: SocketAddr::new(Ipv4Addr::new(127, 0, 0, 1).into(), 0),
            max_database_cache_size: Some(MAX_DATABASE_CACHE_SIZE),
            database_type: DbType::InMemory,
            utxo_validation: true,
            debug: true,
            block_production: Trigger::Instant,
            vm_backtrace: false,
            silent: true,
            static_gas_price: 1,
        }
    }
}
