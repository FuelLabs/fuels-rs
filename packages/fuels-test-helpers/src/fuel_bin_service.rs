use std::{
    net::SocketAddr,
    path::PathBuf,
    process::Stdio,
    time::Duration,
};

use fuel_core_chain_config::{ChainConfig, SnapshotWriter, StateConfig};
use fuel_core_client::client::FuelClient;
use fuel_core_services::State;
use fuel_core_types::blockchain::header::LATEST_STATE_TRANSITION_VERSION;
use fuels_core::{error, types::errors::Result as FuelResult};
use tempfile::{TempDir, tempdir};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::Command,
    spawn,
    task::JoinHandle,
    time::sleep,
};

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
        let mut args = vec![
            "run".to_string(),
            "--ip".to_string(),
            "127.0.0.1".to_string(),
            "--port".to_string(),
            "0".to_string(),
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
            Trigger::Open { period } => {
                args.push(format!("--poa-open-period={}ms", period.as_millis()));
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
                (
                    self.node_config.historical_execution,
                    "--historical-execution",
                ),
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
        let extended_config = ExtendedConfig {
            node_config,
            state_config,
            chain_config,
            snapshot_dir: tempdir()?,
        };

        let (bound_address, handle) = run_node(extended_config).await?;
        server_health_check(bound_address).await?;

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

async fn run_node(
    extended_config: ExtendedConfig,
) -> FuelResult<(SocketAddr, JoinHandle<()>)> {
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

    let mut child = Command::new(path)
        .args(args)
        .kill_on_drop(true)
        .env_clear()
        .env("RUST_LOG", "info")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| error!(Other, "could not spawn `{binary_name}`: {e}"))?;

    let stderr = child.stderr.take().expect("stderr is piped");
    let mut stderr_reader = BufReader::new(stderr).lines();

    let bound_address = tokio::time::timeout(Duration::from_secs(30), async {
        while let Some(line) = stderr_reader
            .next_line()
            .await
            .map_err(|e| error!(Other, "failed to read fuel-core stderr: {e}"))?
        {
            if let Some(addr_str) = line
                .find("Binding GraphQL provider to ")
                .map(|i| &line[i + "Binding GraphQL provider to ".len()..])
            {
                let bound_address: SocketAddr = addr_str.parse().map_err(|e| {
                    error!(
                        Other,
                        "failed to parse bound address '{addr_str}': {e}"
                    )
                })?;
                return Ok(bound_address);
            }
        }
        Err(error!(
            Other,
            "fuel-core process exited before reporting its bound address"
        ))
    })
    .await
    .map_err(|_| {
        error!(
            Other,
            "timed out waiting for fuel-core to report its bound address"
        )
    })??;

    let join_handle = spawn(async move {
        // ensure drop is not called on the tmp dir and it lives throughout the lifetime of the node
        let _unused = tempdir;

        // Buffer all stderr output and dump at once when the process exits
        let mut logs = Vec::new();
        while let Ok(Some(line)) = stderr_reader.next_line().await {
            logs.push(line);
        }

        let status = child
            .wait()
            .await
            .expect("error: could not wait for `fuel-core` process");

        eprintln!("--- fuel-core logs (exit status: {status}) ---");
        for line in &logs {
            eprintln!("{line}");
        }
        eprintln!("--- end fuel-core logs ---");
    });

    Ok((bound_address, join_handle))
}
