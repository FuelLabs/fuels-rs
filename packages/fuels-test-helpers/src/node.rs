use anyhow::{bail, Error as AnyError};
use std::fmt;
use std::io::Write;
use std::net::{Ipv4Addr, SocketAddr};
use std::time::Duration;
use tokio::sync::oneshot;

use portpicker::is_free;
use portpicker::pick_unused_port;

use fuel_core_interfaces::model::BlockHeight;
use fuel_core_interfaces::model::Coin;
use fuel_gql_client::client::FuelClient;
use fuel_gql_client::fuel_tx::{ConsensusParameters, UtxoId};
use fuel_gql_client::fuel_vm::consts::WORD_SIZE;
use fuel_types::{Address, AssetId, Bytes32, Word};
use serde::de::Error;
use serde::{Deserialize, Serialize};
use serde::{Deserializer, Serializer};
use serde_json::{json, Value};
use serde_with::{serde_as, skip_serializing_none};
use serde_with::{DeserializeAs, SerializeAs};
use std::process::Stdio;
use tempfile::NamedTempFile;
use tokio::process::Command;

#[derive(Clone, Copy, Debug)]
pub struct Config {
    pub addr: SocketAddr,
    pub utxo_validation: bool,
    pub predicates: bool,
    pub manual_blocks_enabled: bool,
    pub vm_backtrace: bool,
    pub silent: bool,
}

impl Config {
    pub fn local_node() -> Self {
        Self {
            addr: SocketAddr::new(Ipv4Addr::new(127, 0, 0, 1).into(), 0),
            utxo_validation: false,
            predicates: false,
            manual_blocks_enabled: false,
            vm_backtrace: false,
            silent: true,
        }
    }
}

#[skip_serializing_none]
#[serde_as]
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct CoinConfig {
    #[serde_as(as = "Option<HexType>")]
    #[serde(default)]
    pub tx_id: Option<Bytes32>,
    #[serde_as(as = "Option<HexNumber>")]
    #[serde(default)]
    pub output_index: Option<u64>,
    #[serde_as(as = "Option<HexNumber>")]
    #[serde(default)]
    pub block_created: Option<BlockHeight>,
    #[serde_as(as = "Option<HexNumber>")]
    #[serde(default)]
    pub maturity: Option<BlockHeight>,
    #[serde_as(as = "HexType")]
    pub owner: Address,
    #[serde_as(as = "HexNumber")]
    pub amount: u64,
    #[serde_as(as = "HexType")]
    pub asset_id: AssetId,
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

pub mod serde_hex {
    use core::fmt;
    use std::convert::TryFrom;

    use hex::{FromHex, ToHex};
    use serde::de::Error;
    use serde::{Deserializer, Serializer};

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
                    "value cant exceed {} bytes",
                    WORD_SIZE
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
        let number: u64 = (*value).into();
        HexNumber::serialize_as(&number, serializer)
    }
}

impl<'de> DeserializeAs<'de, BlockHeight> for HexNumber {
    fn deserialize_as<D>(deserializer: D) -> Result<BlockHeight, D::Error>
    where
        D: Deserializer<'de>,
    {
        let number: u64 = HexNumber::deserialize_as(deserializer)?;
        Ok(number.into())
    }
}

pub fn get_node_config_json(
    coins: Vec<(UtxoId, Coin)>,
    consensus_parameters_config: Option<ConsensusParameters>,
) -> Value {
    let coin_configs: Vec<Value> = coins
        .into_iter()
        .map(|(utxo_id, coin)| {
            serde_json::to_value(&CoinConfig {
                tx_id: Some(*utxo_id.tx_id()),
                output_index: Some(utxo_id.output_index() as u64),
                block_created: Some(coin.block_created),
                maturity: Some(coin.maturity),
                owner: coin.owner,
                amount: coin.amount,
                asset_id: coin.asset_id,
            })
            .unwrap()
        })
        .collect();

    let result = serde_json::to_string(&coin_configs).expect("Failed to stringify coins vector");

    let coins: Value =
        serde_json::from_str(result.as_str()).expect("Failed to build config_with_coins JSON");

    let consensus_parameters =
        serde_json::to_value(consensus_parameters_config.unwrap_or_default())
            .expect("Failed to build transaction_parameters JSON");

    let config = json!({
      "chain_name": "local_testnet",
      "block_production": "Instant",
      "parent_network": {
        "type": "LocalTest"
      },
      "initial_state": {
        "coins": coins
      },
      "transaction_parameters": consensus_parameters
    });

    config
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
    coins: Vec<(UtxoId, Coin)>,
    consensus_parameters_config: Option<ConsensusParameters>,
    config: Config,
) {
    // Create a new one-shot channel for sending single values across asynchronous tasks.
    let (tx, rx) = oneshot::channel();

    tokio::spawn(async move {
        let config_json = get_node_config_json(coins, consensus_parameters_config);
        let temp_config_file = write_temp_config_file(config_json);

        let port = &config.addr.port().to_string();
        let mut args = vec![
            "run", // `fuel-core` is now run with `fuel-core run`
            "--ip",
            "127.0.0.1",
            "--port",
            port,
            "--db-type",
            "in-memory",
            "--chain",
            temp_config_file.path().to_str().unwrap(),
        ];

        if config.utxo_validation {
            args.push("--utxo-validation");
        }

        if config.predicates {
            args.push("--predicates");
        }

        if config.manual_blocks_enabled {
            args.push("--manual_blocks_enabled");
        }

        if config.vm_backtrace {
            args.push("--vm-backtrace");
        }

        // Warn if there is more than one binary in PATH.
        let binary_name = "fuel-core";
        let paths = which::which_all(binary_name)
            .unwrap_or_else(|_| panic!("failed to list '{}' binaries", binary_name))
            .collect::<Vec<_>>();
        let path = paths
            .first()
            .unwrap_or_else(|| panic!("no '{}' in PATH", binary_name));
        if paths.len() > 1 {
            tracing::warn!(
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
        let mut running_node = command
            .args(args)
            .kill_on_drop(true)
            .spawn()
            .expect("error: Couldn't read fuel-core: No such file or directory. Please check if fuel-core library is installed. \
        Try this https://fuellabs.github.io/sway/latest/introduction/installation.html");

        let client = FuelClient::from(config.addr);
        server_health_check(&client).await;
        // Sending single to RX to inform that the fuel core node is ready.
        tx.send(()).unwrap();

        running_node.wait().await
    });
    // Awaiting a signal from Tx that informs us if the fuel-core node is ready.
    rx.await.unwrap();
}

pub async fn server_health_check(client: &FuelClient) {
    let mut attempts = 5;
    let mut healthy = client.health().await.unwrap_or(false);

    while attempts > 0 && !healthy {
        healthy = client.health().await.unwrap_or(false);
        tokio::time::sleep(Duration::from_millis(100)).await;
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
    pub async fn new_node(config: Config) -> Result<Self, AnyError> {
        let requested_port = config.addr.port();

        let bound_address = if requested_port == 0 {
            get_socket_address()
        } else if is_free(requested_port) {
            config.addr
        } else {
            bail!("Error: Address already in use");
        };

        new_fuel_node(
            vec![],
            None,
            Config {
                addr: bound_address,
                ..config
            },
        )
        .await;

        Ok(FuelService { bound_address })
    }
}
