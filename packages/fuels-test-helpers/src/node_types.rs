use std::{
    net::{Ipv4Addr, SocketAddr},
    path::PathBuf,
    time::Duration,
};

pub use fuel_core_chain_config::ChainConfig;
pub use fuel_core_chain_config::StateConfig;
use fuel_tx::Word;
use fuel_types::{bytes::WORD_SIZE, BlockHeight};
use fuels_core::types::errors::Result;
use serde::{de::Error as SerdeError, Deserializer, Serializer};
use serde_with::{DeserializeAs, SerializeAs};
use tempfile::TempDir;

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
    #[cfg(not(feature = "fuel-core-lib"))]
    pub fn args_vec(&self) -> Result<Vec<String>> {
        let port = self.node_config.addr.port().to_string();
        let mut args = vec![
            "run".to_string(), // `fuel-core` is now run with `fuel-core run`
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

    #[cfg(not(feature = "fuel-core-lib"))]
    pub fn write_temp_snapshot_files(self) -> Result<TempDir> {
        use fuel_core_chain_config::SnapshotWriter;
        use fuels_core::error;

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
    fn service_config(self) -> fuel_core::Config {
        use fuel_core_chain_config::SnapshotReader;

        let snapshot_reader = SnapshotReader::new_in_memory(self.chain_config, self.state_config);

        Self {
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
            ..fuel_core::service::Config::local_node()
        }
    }
}

#[cfg(feature = "fuel-core-lib")]
impl From<NodeConfig> for fuel_core::service::Config {
    fn from(value: NodeConfig) -> Self {
        Self {
            addr: value.addr,
            max_database_cache_size: value
                .max_database_cache_size
                .unwrap_or(MAX_DATABASE_CACHE_SIZE),
            database_path: match &value.database_type {
                DbType::InMemory => Default::default(),
                DbType::RocksDb(path) => path.clone().unwrap_or_default(),
            },
            database_type: value.database_type.into(),
            utxo_validation: value.utxo_validation,
            debug: value.debug,
            block_production: value.block_production.into(),
            chain_conf: value.chain_conf,
            ..fuel_core::service::Config::local_node()
        }
    }
}

pub(crate) mod serde_hex {
    use std::{convert::TryFrom, fmt};

    use hex::{FromHex, ToHex};
    use serde::{de::Error, Deserializer, Serializer};

    pub fn serialize<T, S>(target: T, ser: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        T: ToHex,
    {
        let s = format!("0x{}", target.encode_hex::<String>());
        ser.serialize_str(&s)
    }

    pub fn deserialize<'de, T, E, D>(des: D) -> Result<T, D::Error>
    where
        D: Deserializer<'de>,
        for<'a> T: TryFrom<&'a [u8], Error = E>,
        E: fmt::Display,
    {
        let raw_string: String = serde::Deserialize::deserialize(des)?;
        let stripped_prefix = raw_string.trim_start_matches("0x");
        let bytes: Vec<u8> = FromHex::from_hex(stripped_prefix).map_err(D::Error::custom)?;
        let result = T::try_from(bytes.as_slice()).map_err(D::Error::custom)?;
        Ok(result)
    }
}

pub(crate) struct HexNumber;

impl SerializeAs<u64> for HexNumber {
    fn serialize_as<S>(value: &u64, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let bytes = value.to_be_bytes();
        serde_hex::serialize(bytes, serializer)
    }
}

impl<'de> DeserializeAs<'de, Word> for HexNumber {
    fn deserialize_as<D>(deserializer: D) -> std::result::Result<Word, D::Error>
    where
        D: Deserializer<'de>,
    {
        let mut bytes: Vec<u8> = serde_hex::deserialize(deserializer)?;
        match bytes.len() {
            len if len > WORD_SIZE => {
                return Err(D::Error::custom(format!(
                    "value can't exceed {WORD_SIZE} bytes",
                )));
            }
            len if len < WORD_SIZE => {
                // pad if length < word size
                bytes = (0..WORD_SIZE - len).map(|_| 0u8).chain(bytes).collect();
            }
            _ => {}
        }
        // We've already verified the bytes.len == WORD_SIZE, force the conversion here.
        Ok(Word::from_be_bytes(
            bytes.try_into().expect("byte lengths checked"),
        ))
    }
}

impl SerializeAs<BlockHeight> for HexNumber {
    fn serialize_as<S>(value: &BlockHeight, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let number = u32::from(*value) as u64;
        HexNumber::serialize_as(&number, serializer)
    }
}

impl<'de> DeserializeAs<'de, BlockHeight> for HexNumber {
    fn deserialize_as<D>(deserializer: D) -> std::result::Result<BlockHeight, D::Error>
    where
        D: Deserializer<'de>,
    {
        let number: u64 = HexNumber::deserialize_as(deserializer)?;
        Ok((number as u32).into())
    }
}
