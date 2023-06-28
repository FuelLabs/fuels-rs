use std::{
    fmt,
    io::Write,
    net::{Ipv4Addr, SocketAddr},
    process::Stdio,
    time::Duration,
};

pub use fuel_core_chain_config::ChainConfig;
use fuel_core_chain_config::StateConfig;
use fuel_core_client::client::FuelClient;
use fuel_types::BlockHeight;
use fuel_types::Word;
use fuel_vm::consts::WORD_SIZE;
use fuels_core::types::{
    coin::Coin,
    errors::{error, Error},
    message::Message,
};
use portpicker::{is_free, pick_unused_port};
use serde::{de::Error as SerdeError, Deserializer, Serializer};
use serde_json::Value;
use serde_with::{DeserializeAs, SerializeAs};
use tempfile::NamedTempFile;
use tokio::{process::Command, sync::oneshot};

use crate::utils::{into_coin_configs, into_message_configs};

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

#[derive(Clone, Debug)]
pub struct Config {
    pub addr: SocketAddr,
    pub utxo_validation: bool,
    pub manual_blocks_enabled: bool,
    pub block_production: Trigger,
    pub vm_backtrace: bool,
    pub silent: bool,
}

impl Config {
    pub fn local_node() -> Self {
        Self {
            addr: SocketAddr::new(Ipv4Addr::new(127, 0, 0, 1).into(), 0),
            utxo_validation: false,
            manual_blocks_enabled: false,
            block_production: Trigger::Instant,
            vm_backtrace: false,
            silent: true,
        }
    }
}

pub type InternalDaBlockHeight = u64;

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

pub mod serde_hex {
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
                bytes = (0..WORD_SIZE - len)
                    .map(|_| 0u8)
                    .chain(bytes.into_iter())
                    .collect();
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

pub fn get_node_config_json(
    coins: Vec<Coin>,
    messages: Vec<Message>,
    chain_config: Option<ChainConfig>,
) -> Value {
    let coin_configs = into_coin_configs(coins);
    let messages = into_message_configs(messages);

    let mut chain_config = chain_config.unwrap_or_else(ChainConfig::local_testnet);

    chain_config.initial_state = Some(StateConfig {
        coins: Some(coin_configs),
        contracts: None,
        messages: Some(messages),
        height: None,
    });

    serde_json::to_value(&chain_config).expect("Failed to build `ChainConfig` JSON")
}

fn write_temp_config_file(config: Value) -> NamedTempFile {
    let config_file = NamedTempFile::new();

    let _ = writeln!(
        config_file.as_ref().unwrap().as_file(),
        "{}",
        &config.to_string()
    );

    config_file.unwrap()
}

pub async fn new_fuel_node(
    coins: Vec<Coin>,
    messages: Vec<Message>,
    config: Config,
    chain_config: Option<ChainConfig>,
) {
    // Create a new one-shot channel for sending single values across asynchronous tasks.
    let (tx, rx) = oneshot::channel();

    tokio::spawn(async move {
        let config_json = get_node_config_json(coins, messages, chain_config);
        let temp_config_file = write_temp_config_file(config_json);

        let port = config.addr.port().to_string();
        let mut args = vec![
            "run".to_string(), // `fuel-core` is now run with `fuel-core run`
            "--ip".to_string(),
            "127.0.0.1".to_string(),
            "--port".to_string(),
            port,
            "--db-type".to_string(),
            "in-memory".to_string(),
            "--chain".to_string(),
            temp_config_file.path().to_str().unwrap().to_string(),
        ];

        if config.utxo_validation {
            args.push("--utxo-validation".to_string());
        }

        if config.manual_blocks_enabled {
            args.push("--manual_blocks_enabled".to_string());
        }

        match config.block_production {
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
            Trigger::Hybrid {
                min_block_time,
                max_tx_idle_time,
                max_block_time,
            } => {
                args.push(format!(
                    "--poa-hybrid-min-time={}ms",
                    min_block_time.as_millis()
                ));
                args.push(format!(
                    "--poa-hybrid-idle-time={}ms",
                    max_tx_idle_time.as_millis()
                ));
                args.push(format!(
                    "--poa-hybrid-max-time={}ms",
                    max_block_time.as_millis()
                ));
            }
        };

        if config.vm_backtrace {
            args.push("--vm-backtrace".to_string());
        }

        // Warn if there is more than one binary in PATH.
        let binary_name = "fuel-core";
        let paths = which::which_all(binary_name)
            .unwrap_or_else(|_| panic!("failed to list '{binary_name}' binaries"))
            .collect::<Vec<_>>();
        let path = paths
            .first()
            .unwrap_or_else(|| panic!("no '{binary_name}' in PATH"));
        if paths.len() > 1 {
            eprintln!(
                "found more than one '{}' binary in PATH, using '{}'",
                binary_name,
                path.display()
            );
        }

        let mut command = Command::new(path);
        command.stdin(Stdio::null());
        if config.silent {
            command.stdout(Stdio::null()).stderr(Stdio::null());
        }
        let running_node = command.args(args).kill_on_drop(true).env_clear().output();

        let client = FuelClient::from(config.addr);
        server_health_check(&client).await;
        // Sending single to RX to inform that the fuel core node is ready.
        tx.send(()).unwrap();

        let result = running_node
            .await
            .expect("error: Couldn't find fuel-core in PATH.");
        let stdout = String::from_utf8_lossy(&result.stdout);
        let stderr = String::from_utf8_lossy(&result.stderr);
        eprintln!("the exit status from the fuel binary was: {result:?}, stdout: {stdout}, stderr: {stderr}");
    });
    // Awaiting a signal from Tx that informs us if the fuel-core node is ready.
    rx.await.unwrap();
}

pub async fn server_health_check(client: &FuelClient) {
    let mut attempts = 5;
    let mut healthy = client.health().await.unwrap_or(false);
    let between_attempts = Duration::from_millis(300);

    while attempts > 0 && !healthy {
        healthy = client.health().await.unwrap_or(false);
        tokio::time::sleep(between_attempts).await;
        attempts -= 1;
    }

    if !healthy {
        panic!("error: Could not connect to fuel core server.")
    }
}

pub fn get_socket_address() -> SocketAddr {
    let free_port = pick_unused_port().expect("No ports free");
    SocketAddr::new("127.0.0.1".parse().unwrap(), free_port)
}

pub struct FuelService {
    pub bound_address: SocketAddr,
}

impl FuelService {
    pub async fn new_node(config: Config) -> Result<Self, Error> {
        let requested_port = config.addr.port();

        let bound_address = if requested_port == 0 {
            get_socket_address()
        } else if is_free(requested_port) {
            config.addr
        } else {
            return Err(error!(InfrastructureError, "Error: Address already in use"));
        };

        new_fuel_node(
            vec![],
            vec![],
            Config {
                addr: bound_address,
                ..config
            },
            None,
        )
        .await;

        Ok(FuelService { bound_address })
    }
}
