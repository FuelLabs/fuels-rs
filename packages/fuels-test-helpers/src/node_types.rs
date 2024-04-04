use std::{
    net::{Ipv4Addr, SocketAddr},
    path::PathBuf,
    time::Duration,
};
use tempfile::TempDir;

use fuels_core::error;
use fuels_core::types::errors::Result as FuelResult;

use fuel_core_chain_config::SnapshotWriter;
pub use fuel_core_chain_config::{ChainConfig, StateConfig};

#[cfg(feature = "fuel-core-lib")]
use fuel_core::service::Config as ServiceConfig;

const MAX_DATABASE_CACHE_SIZE: usize = 10 * 1024 * 1024;

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
        }
    }
}

#[derive(Debug)]
pub(crate) struct ExtendedConfig {
    pub node_config: NodeConfig,
    pub chain_config: ChainConfig,
    pub state_config: StateConfig,
    pub snapshot_dir: TempDir,
}

impl ExtendedConfig {
    pub fn args_vec(&self) -> fuels_core::types::errors::Result<Vec<String>> {
        let port = self.node_config.addr.port().to_string();
        let mut args = vec![
            "run".to_string(),
            "--ip".to_string(),
            "127.0.0.1".to_string(),
            "--port".to_string(),
            port,
            "--snapshot".to_string(),
            self.snapshot_dir
                .path()
                .to_str()
                .expect("Failed to find config file")
                .to_string(),
        ];

        args.push("--db-type".to_string());
        match &self.node_config.database_type {
            DbType::InMemory => args.push("in-memory".to_string()),
            DbType::RocksDb(path_to_db) => {
                args.push("rocks-db".to_string());
                let path = path_to_db.as_ref().cloned().unwrap_or_else(|| {
                    PathBuf::from(std::env::var("HOME").expect("HOME env var missing"))
                        .join(".fuel/db")
                });
                args.push("--db-path".to_string());
                args.push(path.to_string_lossy().to_string());
            }
        }

        if let Some(cache_size) = self.node_config.max_database_cache_size {
            args.push("--max-database-cache-size".to_string());
            args.push(cache_size.to_string());
        }

        match self.node_config.block_production {
            Trigger::Instant => {
                args.push("--poa-instant=true".to_string());
            }
            Trigger::Never => {
                args.push("--poa-instant=false".to_string());
            }
            Trigger::Interval { block_time } => {
                args.push(format!(
                    "--poa-interval-period={}ms",
                    block_time.as_millis()
                ));
            }
        };

        args.extend(
            [
                (self.node_config.vm_backtrace, "--vm-backtrace"),
                (self.node_config.utxo_validation, "--utxo-validation"),
                (self.node_config.debug, "--debug"),
            ]
            .into_iter()
            .filter(|(flag, _)| *flag)
            .map(|(_, arg)| arg.to_string()),
        );

        Ok(args)
    }

    pub fn write_temp_snapshot_files(self) -> FuelResult<TempDir> {
        let mut writer = SnapshotWriter::json(self.snapshot_dir.path());
        writer
            .write_chain_config(&self.chain_config)
            .map_err(|e| error!(Other, "could not write chain config: {}", e))?;
        writer
            .write_state_config(self.state_config)
            .map_err(|e| error!(Other, "could not write state config: {}", e))?;

        Ok(self.snapshot_dir)
    }

    #[cfg(feature = "fuel-core-lib")]
    pub fn service_config(self) -> ServiceConfig {
        use fuel_core_chain_config::SnapshotReader;

        let snapshot_reader = SnapshotReader::new_in_memory(self.chain_config, self.state_config);

        ServiceConfig {
            addr: self.node_config.addr,
            max_database_cache_size: self
                .node_config
                .max_database_cache_size
                .unwrap_or(MAX_DATABASE_CACHE_SIZE),
            database_path: match &self.node_config.database_type {
                DbType::InMemory => Default::default(),
                DbType::RocksDb(path) => path.clone().unwrap_or_default(),
            },
            database_type: self.node_config.database_type.into(),
            utxo_validation: self.node_config.utxo_validation,
            debug: self.node_config.debug,
            block_production: self.node_config.block_production.into(),
            ..ServiceConfig::local_node()
        }
    }
}
