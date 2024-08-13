use std::{net::IpAddr, path::PathBuf, time::Duration};

use fuels_core::types::errors::Result;

pub struct FuelNodeCommand {}

impl FuelNodeCommand {
    pub fn run() -> Run {
        Run {}
    }
}

#[derive(Debug, Clone, Default)]
pub enum DbType {
    #[default]
    InMemory,
    RocksDb {
        db_path: String,
        /// Prunes the db. Genesis is done from the provided snapshot or the local testnet configuration
        prune: bool,
        /// Defines the state rewind policy for the database when RocksDB is enabled
        state_rewind_duration: Option<Duration>,
    },
}

#[derive(Debug, Clone, Default)]
pub struct DbConfig {
    /// The maximum database cache size in bytes
    pub cache_size: Option<u64>,
    pub db_type: DbType,
    pub prune: bool,
}

#[derive(Debug, Clone, Default)]
pub enum PoaConfig {
    #[default]
    /// Use instant block production mode. Newly submitted txs will immediately trigger the production of the next block.
    Instant,
    /// Interval trigger option. Produces blocks on a fixed interval regardless of txpool activity.
    Interval { period: Duration },
}

pub struct Run {
    service_name: Option<String>,
    db_config: Option<DbConfig>,
    snapshot: Option<PathBuf>,
    continue_services_on_error: Option<bool>,
    debug: Option<bool>,
    vm_backtrace: Option<bool>,
    utxo_validation: Option<bool>,
    native_executor_version: Option<String>,
    gas_price: Option<GasPrice>,
    consensus_key: Option<String>,
    poa_config: Option<PoaConfig>,
    coinbase_recipient: Option<String>,
    tx_pool_config: Option<TxPoolConfig>,
    ip: Option<IpAddr>,
    port: Option<u16>,
    gql_config: Option<GqlConfig>,
    relayer_config: Option<RelayerConfig>,
    enable_metrics: Option<bool>,
    sync_config: Option<SyncConfig>,
    memory_pool_size: Option<u64>,
}

#[derive(Debug, Clone, Default)]
pub struct GasPrice {
    /// The starting gas price for the network
    pub starting_gas_price: Option<u64>,
    /// The percentage change in gas price per block
    pub gas_price_change_percent: Option<u64>,
    pub min_gas_price: Option<u64>,
    /// The percentage threshold for gas price increase
    pub gas_price_threshold_percent: Option<u64>,
}

#[derive(Debug, Clone, Default)]
pub struct Blacklist {
    pub coins: Option<Vec<String>>,
    pub messages: Option<Vec<String>>,
    pub contracts: Option<Vec<String>>,
    pub addresses: Option<Vec<String>>,
}

#[derive(Debug, Clone, Default)]
pub struct TxPoolConfig {
    /// The max time to live of the transaction inside of the `TxPool`
    ttl: Option<Duration>,
    /// The max number of simultaneously stored transactions
    max_number: Option<u64>,
    /// The max depth of the dependent transactions
    max_depth: Option<u64>,
    /// The max number of active subscriptions
    max_active_subscriptions: Option<u64>,
    /// The list of banned addresses ignored by the pool
    blacklist: Option<Blacklist>,
}

pub struct GqlConfig {
    /// The max depth of GraphQL queries
    pub max_depth: Option<u64>,
    /// The max complexity of GraphQL queries
    pub max_complexity: Option<u64>,
    /// The max recursive depth of GraphQL queries
    pub max_recursive_depth: Option<u64>,
    /// The max body limit of the GraphQL query
    pub request_body_bytes_limit: Option<u64>,
    /// Time to wait after submitting a query before debug info will be logged about query
    pub query_log_threshold_time: Option<Duration>,
    /// Timeout before the request is dropped
    pub request_timeout: Option<Duration>,
}

#[derive(Debug, Clone, Default)]
pub struct RelayerConfig {
    /// Enable the Relayer
    pub enable_relayer: Option<bool>,
    /// Uri address to ethereum client. It can be in format of `http://localhost:8545/` or `ws://localhost:8545/`. If not set relayer will not start.
    pub url: Option<String>,
    /// Ethereum contract address.
    pub v2_listening_contracts: Option<String>,
    /// Number of da block that the contract is deployed at
    pub da_deploy_height: Option<u64>,
    /// Number of pages or blocks containing logs that should be downloaded in a single call to the da layer
    pub log_page_size: Option<u64>,
    /// The minimum number of seconds that the relayer polling loop will take before running again. If this is too low the DA layer risks being spammed
    pub sync_minimum_duration: Option<Duration>,
    pub syncing_call_frequency: Option<Duration>,
    pub syncing_log_frequency: Option<Duration>,
    pub verify_max_wait_time: Option<Duration>,
    pub verify_max_da_lag: Option<u64>,
}

#[derive(Debug, Clone, Default)]
pub struct SyncConfig {
    /// The number of reserved peers to connect to before starting to sync
    pub min_connected_reserved_peers: Option<u64>,
    /// Time to wait after receiving the latest block before considered to be Synced
    pub time_until_synced: Option<Duration>,
}

#[derive(Debug, Clone, Default)]
pub struct PyroscopeConfig {
    /// Enables realtime profiling with pyroscope if set, and streams results to the pyroscope endpoint. For best results, the binary should be built with debug symbols included
    pub url: Option<String>,
    /// Pyroscope sample frequency in hertz. A higher sample rate improves profiling granularity at the cost of additional measurement overhead
    pub pprof_sample_rate: Option<u64>,
}

impl Run {
    pub async fn start(self) -> Result<FuelNodeInstance> {
        let cmd = vec!["fuel-core".to_string(), "run".to_string()];
        Ok(FuelNodeInstance { cmd })
    }

    /// Vanity name for node, used in telemetry
    pub fn with_service_name(self, service_name: impl Into<String>) -> Run {
        Run {
            service_name: Some(service_name.into()),
            ..self
        }
    }

    pub fn with_db_config(self, config: DbConfig) -> Run {
        Self {
            db_config: Some(config),
            ..self
        }
    }

    /// Snapshot from which to do (re)genesis. Defaults to local testnet configuration
    pub fn with_snapshot(self, snapshot: impl Into<PathBuf>) -> Run {
        Run {
            snapshot: Some(snapshot.into()),
            ..self
        }
    }

    /// The determines whether to continue the services on internal error or not
    pub fn with_continue_services_on_error(self, continue_services_on_error: bool) -> Run {
        Run {
            continue_services_on_error: Some(continue_services_on_error),
            ..self
        }
    }

    /// Enables debug mode: - Allows GraphQL Endpoints to arbitrarily advance blocks. - Enables debugger GraphQL Endpoints. - Allows setting `utxo_validation` to `false`
    pub fn with_debug(self, debug: bool) -> Run {
        Run {
            debug: Some(debug),
            ..self
        }
    }

    /// Enable logging of backtraces from vm errors
    pub fn with_vm_backtrace(self, vm_backtrace: bool) -> Run {
        Run {
            vm_backtrace: Some(vm_backtrace),
            ..self
        }
    }

    /// Enable full utxo stateful validation disabled by default until downstream consumers stabilize
    pub fn with_utxo_validation(self, utxo_validation: bool) -> Run {
        Run {
            utxo_validation: Some(utxo_validation),
            ..self
        }
    }

    /// Overrides the version of the native executor
    pub fn with_native_executor_version(self, native_executor_version: impl Into<String>) -> Run {
        Run {
            native_executor_version: Some(native_executor_version.into()),
            ..self
        }
    }

    /// Gas price configuration
    pub fn with_gas_price(self, gas_price: GasPrice) -> Run {
        Run {
            gas_price: Some(gas_price),
            ..self
        }
    }

    /// The signing key used when producing blocks
    pub fn with_consensus_key(self, consensus_key: impl Into<String>) -> Run {
        Run {
            consensus_key: Some(consensus_key.into()),
            ..self
        }
    }

    pub fn with_poa_config(self, poa_config: PoaConfig) -> Run {
        Self { ..self }
    }

    /// The block's fee recipient public key
    pub fn with_coinbase_recipient(self, coinbase_recipient: impl Into<String>) -> Run {
        Run {
            coinbase_recipient: Some(coinbase_recipient.into()),
            ..self
        }
    }

    pub fn with_tx_pool_config(self, tx_pool_config: TxPoolConfig) -> Run {
        Self {
            tx_pool_config: Some(tx_pool_config),
            ..self
        }
    }

    /// The IP address to bind the GraphQL service to
    pub fn with_ip(self, ip: IpAddr) -> Run {
        Run {
            ip: Some(ip),
            ..self
        }
    }

    /// The port to bind the GraphQL service to.
    pub fn with_port(self, port: u16) -> Run {
        Run {
            port: Some(port),
            ..self
        }
    }

    pub fn with_gql_config(self, gql_config: GqlConfig) -> Run {
        Run {
            gql_config: Some(gql_config),
            ..self
        }
    }

    pub fn with_relayer_config(self, relayer_config: RelayerConfig) -> Run {
        Run {
            relayer_config: Some(relayer_config),
            ..self
        }
    }

    pub fn with_enable_metrics(self, enable_metrics: bool) -> Run {
        Run {
            enable_metrics: Some(enable_metrics),
            ..self
        }
    }

    pub fn with_sync_config(self, sync_config: SyncConfig) -> Run {
        Run {
            sync_config: Some(sync_config),
            ..self
        }
    }

    /// The size of the memory pool in number of `MemoryInstance`s [env: MEMORY_POOL_SIZE=] [default: 32]
    pub fn with_memory_pool_size(self, memory_pool_size: u64) -> Run {
        Run {
            memory_pool_size: Some(memory_pool_size),
            ..self
        }
    }
}

pub struct FuelNodeInstance {
    cmd: Vec<String>,
}
