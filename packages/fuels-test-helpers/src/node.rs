use std::borrow::Borrow;
use std::fmt;
use std::io::Write;
use std::net::{Ipv4Addr, SocketAddr};
use std::time::Duration;

use fuel_core_interfaces::model::BlockHeight;
use fuel_gql_client::client::FuelClient;
use fuel_gql_client::fuel_vm::consts::WORD_SIZE;
use fuel_types::{Address, AssetId, Bytes32, Word};
use portpicker::Port;
use serde::de::Error;
use serde::{Deserialize, Serialize};
use serde::{Deserializer, Serializer};
use serde_json::{json, Value};
use serde_with::{serde_as, skip_serializing_none};
use serde_with::{DeserializeAs, SerializeAs};
use std::process::Stdio;
use tempfile::NamedTempFile;
use tokio::process::Command;

#[derive(Clone, Debug)]
pub struct Config {
    pub addr: SocketAddr,
}

impl Config {
    pub fn local_node() -> Self {
        Self {
            addr: SocketAddr::new(Ipv4Addr::new(127, 0, 0, 1).into(), 0),
        }
    }
}

#[skip_serializing_none]
#[serde_as]
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
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

pub fn get_node_config_json(coins: Value) -> NamedTempFile {
    let config = json!({
      "chain_name": "local_testnet",
      "block_production": "Instant",
      "parent_network": {
        "type": "LocalTest"
      },
      "initial_state": {
        "coins": coins
      },
      "transaction_parameters": {
        "contract_max_size": 16777216,
        "max_inputs": 255,
        "max_outputs": 255,
        "max_witnesses": 255,
        "max_gas_per_tx": 100000000,
        "max_script_length": 1048576,
        "max_script_data_length": 1048576,
        "max_static_contracts": 255,
        "max_storage_slots": 255,
        "max_predicate_length": 1048576,
        "max_predicate_data_length": 1048576,
        "gas_price_factor": 1000000000,
      }
    });

    let config_file = NamedTempFile::new();

    let _ = writeln!(
        config_file.as_ref().unwrap().as_file(),
        "{}",
        &config.to_string()
    );

    config_file.unwrap()
}

pub fn spawn_fuel_service(config_with_coins: Value, free_port: Port) {
    tokio::spawn(async move {
        let temp_config_file = get_node_config_json(config_with_coins);
        let mut running_node = Command::new("fuel-core")
            .arg("--ip")
            .arg("127.0.0.1")
            .arg("--port")
            .arg(free_port.to_string())
            .arg("--chain")
            .arg(temp_config_file.borrow().path())
            .arg("--db-type")
            .arg("in-memory")
            .kill_on_drop(true)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .spawn()
            .expect("error: Couldn't read fuel-core: No such file or directory. Please check if fuel-core library is installed. \
        Try this https://fuellabs.github.io/sway/latest/introduction/installation.html");

        running_node.wait().await
    });
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
