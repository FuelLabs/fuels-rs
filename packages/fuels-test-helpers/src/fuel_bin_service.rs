use fuels_core::types::errors::{Error, Result as FuelResult};
use tempfile::NamedTempFile;

use fuel_core_client::client::FuelClient;
use fuel_core_services::State;
use std::{net::SocketAddr, path::PathBuf, pin::Pin, time::Duration};

use crate::node_types::{Config, DbType, Trigger};
use portpicker::{is_free, pick_unused_port};
use tokio::{process::Command, spawn, task::JoinHandle, time::sleep};

#[derive(Debug)]
struct ExtendedConfig {
    pub config: Config,
    pub config_file: NamedTempFile,
}

impl ExtendedConfig {
    pub fn config_to_args_vec(&mut self) -> FuelResult<Vec<String>> {
        self.write_temp_chain_config_file()?;

        let port = self.config.addr.port().to_string();
        let mut args = vec![
            "run".to_string(), // `fuel-core` is now run with `fuel-core run`
            "--ip".to_string(),
            "127.0.0.1".to_string(),
            "--port".to_string(),
            port,
            "--chain".to_string(),
            self.config_file
                .path()
                .to_str()
                .expect("Failed to find config file")
                .to_string(),
        ];

        args.push("--db-type".to_string());
        match &self.config.database_type {
            DbType::InMemory => args.push("in-memory".to_string()),
            DbType::RocksDb(path_to_db) => {
                args.push("rocks-db".to_string());
                let path = path_to_db.as_ref().map(Clone::clone).unwrap_or_else(|| {
                    PathBuf::from(std::env::var("HOME").expect("HOME env var missing"))
                        .join(".fuel/db")
                });
                args.push("--db-path".to_string());
                args.push(path.to_string_lossy().to_string());
            }
        }

        if let Some(cache_size) = self.config.max_database_cache_size {
            args.push("--max-database-cache-size".to_string());
            args.push(cache_size.to_string());
        }

        match self.config.block_production {
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

        args.extend(
            [
                (self.config.vm_backtrace, "--vm-backtrace"),
                (self.config.utxo_validation, "--utxo-validation"),
                (self.config.manual_blocks_enabled, "--manual_blocks_enabled"),
            ]
            .into_iter()
            .filter(|(flag, _)| *flag)
            .map(|(_, arg)| arg.to_string()),
        );

        Ok(args)
    }

    pub fn write_temp_chain_config_file(&mut self) -> FuelResult<()> {
        Ok(serde_json::to_writer(
            &mut self.config_file,
            &self.config.chain_conf,
        )?)
    }
}

#[derive(Clone, Default, Debug)]
pub struct SharedState {
    pub config: Config,
}

pub struct ServerParams {
    extended_config: ExtendedConfig,
}

pub struct FuelService {
    pub bound_address: SocketAddr,
    runner: Pin<Box<JoinHandle<()>>>,
}

impl FuelService {
    pub async fn new_node(config: Config) -> FuelResult<Self> {
        let requested_port = config.addr.port();

        let bound_address = match requested_port {
            0 => get_socket_address(),
            _ if is_free(requested_port) => config.addr,
            _ => return Err(Error::IOError(std::io::ErrorKind::AddrInUse.into())),
        };

        let config = Config {
            addr: bound_address,
            ..config
        };

        let params = ServerParams {
            extended_config: ExtendedConfig {
                config,
                config_file: NamedTempFile::new()?,
            },
        };

        let runner = run_node(params).await?;

        Ok(FuelService {
            bound_address,
            runner: Box::pin(runner),
        })
    }

    pub async fn stop(&self) -> FuelResult<State> {
        self.runner.abort();
        Ok(State::Stopped)
    }
}

async fn server_health_check(client: &FuelClient) -> FuelResult<()> {
    let mut attempts = 5;
    let mut healthy = client.health().await.unwrap_or(false);
    let between_attempts = Duration::from_millis(300);

    while attempts > 0 && !healthy {
        healthy = client.health().await.unwrap_or(false);
        sleep(between_attempts).await;
        attempts -= 1;
    }

    if !healthy {
        panic!("error: Could not connect to fuel core server.")
    }

    Ok(())
}

fn get_socket_address() -> SocketAddr {
    let free_port = pick_unused_port().expect("No ports free");
    SocketAddr::new("127.0.0.1".parse().unwrap(), free_port)
}

async fn run_node(params: ServerParams) -> FuelResult<JoinHandle<()>> {
    let ServerParams {
        mut extended_config,
    } = params;
    let args = extended_config.config_to_args_vec()?;

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
    let running_node = command.args(args).kill_on_drop(true).output();

    let address = extended_config.config.addr;
    let client = FuelClient::from(address);
    server_health_check(&client).await?;

    let join_handle = spawn(async move {
        let result = running_node
            .await
            .expect("error: Couldn't find fuel-core in PATH.");
        let stdout = String::from_utf8_lossy(&result.stdout);
        let stderr = String::from_utf8_lossy(&result.stderr);
        eprintln!("the exit status from the fuel binary was: {result:?}, stdout: {stdout}, stderr: {stderr}");
    });

    Ok(join_handle)
}