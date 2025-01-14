use std::{
    net::{IpAddr, SocketAddr},
    path::PathBuf,
    time::Duration,
};

use fuel_core_chain_config::{ChainConfig, SnapshotWriter, StateConfig};
use fuel_core_client::client::FuelClient;
use fuel_core_services::State;
use fuel_core_types::blockchain::header::LATEST_STATE_TRANSITION_VERSION;
use fuels_core::{error, types::errors::Result as FuelResult};
use portpicker::{is_free, pick_unused_port};
use tempfile::{tempdir, TempDir};
use tokio::{process::Command, spawn, task::JoinHandle, time::sleep};

use crate::node_types::{DbType, NodeConfig, Trigger};

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

        let body_limit = self.node_config.graphql_request_body_bytes_limit;
        args.push(format!("--graphql-request-body-bytes-limit={body_limit}"));

        // This ensures forward compatibility when running against a newer node with a different native executor version.
        // If the node detects our older version in the chain configuration, it defaults to using the wasm executor.
        // However, since we don't include a wasm executor, this would lead to code loading failure and a node crash.
        // To prevent this, we force the node to use our version number to refer to its native executor.
        let executor_version = self
            .chain_config
            .genesis_state_transition_version
            .unwrap_or(LATEST_STATE_TRANSITION_VERSION);
        args.push(format!("--native-executor-version={executor_version}"));

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

        args.push(format!(
            "--starting-gas-price={}",
            self.node_config.starting_gas_price
        ));

        Ok(args)
    }

    pub fn write_temp_snapshot_files(self) -> FuelResult<TempDir> {
        let writer = SnapshotWriter::json(self.snapshot_dir.path());
        writer
            .write_state_config(self.state_config, &self.chain_config)
            .map_err(|e| error!(Other, "could not write state config: {}", e))?;

        Ok(self.snapshot_dir)
    }
}

pub struct FuelService {
    pub bound_address: SocketAddr,
    handle: JoinHandle<()>,
}

impl FuelService {
    pub async fn new_node(
        node_config: NodeConfig,
        chain_config: ChainConfig,
        state_config: StateConfig,
    ) -> FuelResult<Self> {
        let requested_port = node_config.addr.port();

        let bound_address = match requested_port {
            0 => get_socket_address()?,
            _ if is_free(requested_port) => node_config.addr,
            _ => {
                return Err(error!(
                    IO,
                    "could not find a free port to start a fuel node"
                ))
            }
        };

        let node_config = NodeConfig {
            addr: bound_address,
            ..node_config
        };

        let extended_config = ExtendedConfig {
            node_config,
            state_config,
            chain_config,
            snapshot_dir: tempdir()?,
        };

        let addr = extended_config.node_config.addr;
        let handle = run_node(extended_config).await?;
        server_health_check(addr).await?;

        Ok(FuelService {
            bound_address,
            handle,
        })
    }

    pub fn stop(&self) -> FuelResult<State> {
        self.handle.abort();
        Ok(State::Stopped)
    }
}

async fn server_health_check(address: SocketAddr) -> FuelResult<()> {
    let client = FuelClient::from(address);

    let mut attempts = 5;
    let mut healthy = client.health().await.unwrap_or(false);
    let between_attempts = Duration::from_millis(300);

    while attempts > 0 && !healthy {
        healthy = client.health().await.unwrap_or(false);
        sleep(between_attempts).await;
        attempts -= 1;
    }

    if !healthy {
        return Err(error!(Other, "could not connect to fuel core server"));
    }

    Ok(())
}

fn get_socket_address() -> FuelResult<SocketAddr> {
    let free_port = pick_unused_port().ok_or(error!(Other, "could not pick a free port"))?;
    let address: IpAddr = "127.0.0.1".parse().expect("is valid ip");

    Ok(SocketAddr::new(address, free_port))
}

async fn run_node(extended_config: ExtendedConfig) -> FuelResult<JoinHandle<()>> {
    let args = extended_config.args_vec()?;
    let tempdir = extended_config.write_temp_snapshot_files()?;

    let binary_name = "fuel-core";

    let paths = which::which_all(binary_name)
        .map_err(|_| error!(Other, "failed to list `{binary_name}` binaries"))?
        .collect::<Vec<_>>();

    let path = paths
        .first()
        .ok_or_else(|| error!(Other, "no `{binary_name}` in PATH"))?;

    if paths.len() > 1 {
        eprintln!(
            "found more than one `{binary_name}` binary in PATH, using `{}`",
            path.display()
        );
    }

    let mut command = Command::new(path);
    let running_node = command.args(args).kill_on_drop(true).output();

    let join_handle = spawn(async move {
        // ensure drop is not called on the tmp dir and it lives throughout the lifetime of the node
        let _unused = tempdir;
        let result = running_node
            .await
            .expect("error: could not find `fuel-core` in PATH`");
        let stdout = String::from_utf8_lossy(&result.stdout);
        let stderr = String::from_utf8_lossy(&result.stderr);
        eprintln!("the exit status from the fuel binary was: {result:?}, stdout: {stdout}, stderr: {stderr}");
    });

    Ok(join_handle)
}
