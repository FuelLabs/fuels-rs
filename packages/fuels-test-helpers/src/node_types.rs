use std::{
    fmt,
    net::{Ipv4Addr, SocketAddr},
    path::PathBuf,
    time::Duration,
};

pub use fuel_core_chain_config::ChainConfig;
use fuel_types::{BlockHeight, Word};
use fuels_core::constants::WORD_SIZE;
use serde::{de::Error as SerdeError, Deserializer, Serializer};
use serde_with::{DeserializeAs, SerializeAs};

const MAX_DATABASE_CACHE_SIZE: usize = 10 * 1024 * 1024;

#[derive(Clone, Debug)]
pub enum Trigger {
    Instant,
    Never,
    Interval {
        block_time: Duration,
    },
    Hybrid {
        min_block_time: Duration,
        max_tx_idle_time: Duration,
        max_block_time: Duration,
    },
}

#[cfg(feature = "fuel-core-lib")]
impl From<Trigger> for fuel_core_poa::Trigger {
    fn from(value: Trigger) -> Self {
        match value {
            Trigger::Instant => fuel_core_poa::Trigger::Instant,
            Trigger::Never => fuel_core_poa::Trigger::Never,
            Trigger::Interval { block_time } => fuel_core_poa::Trigger::Interval { block_time },
            _ => value.into(),
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
pub struct Config {
    pub addr: SocketAddr,
    pub max_database_cache_size: Option<usize>,
    pub database_type: DbType,
    pub utxo_validation: bool,
    pub debug: bool,
    pub block_production: Trigger,
    pub vm_backtrace: bool,
    pub silent: bool,
    pub chain_conf: ChainConfig,
}

impl Default for Config {
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
            chain_conf: ChainConfig::local_testnet(),
        }
    }
}

#[cfg(feature = "fuel-core-lib")]
impl From<Config> for fuel_core::service::Config {
    fn from(value: Config) -> Self {
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

pub(crate) struct HexType;

impl<T: AsRef<[u8]>> SerializeAs<T> for HexType {
    fn serialize_as<S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serde_hex::serialize(value, serializer)
    }
}

impl<'de, T, E> DeserializeAs<'de, T> for HexType
where
    for<'a> T: TryFrom<&'a [u8], Error = E>,
    E: fmt::Display,
{
    fn deserialize_as<D>(deserializer: D) -> Result<T, D::Error>
    where
        D: Deserializer<'de>,
    {
        serde_hex::deserialize(deserializer)
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
    fn serialize_as<S>(value: &u64, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let bytes = value.to_be_bytes();
        serde_hex::serialize(bytes, serializer)
    }
}

impl<'de> DeserializeAs<'de, Word> for HexNumber {
    fn deserialize_as<D>(deserializer: D) -> Result<Word, D::Error>
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
    fn serialize_as<S>(value: &BlockHeight, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let number = u32::from(*value) as u64;
        HexNumber::serialize_as(&number, serializer)
    }
}

impl<'de> DeserializeAs<'de, BlockHeight> for HexNumber {
    fn deserialize_as<D>(deserializer: D) -> Result<BlockHeight, D::Error>
    where
        D: Deserializer<'de>,
    {
        let number: u64 = HexNumber::deserialize_as(deserializer)?;
        Ok((number as u32).into())
    }
}
