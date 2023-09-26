use fuels_core::types::errors::{error, Error, Result as FuelResult};
use tempfile::NamedTempFile;

use fuel_core_services::{RunnableService, RunnableTask, ServiceRunner, State, StateWatcher};

pub use fuel_core_services::Service as ServiceTrait;

use fuel_core_client::client::FuelClient;
use serde_json::{to_value, Value};
use std::{io::Write, net::SocketAddr, path::PathBuf, pin::Pin, time::Duration};

use tokio::{process::Command, spawn, task::JoinHandle, time::sleep};
use crate::node_types::{Config, DbType, Trigger, DEFAULT_CACHE_SIZE};
use portpicker::{is_free, pick_unused_port};

#[derive(Debug)]
struct ExtendedConfig {
    pub config: Config,
    pub config_file: NamedTempFile,
}

impl ExtendedConfig {
    pub fn config_to_args_vec(&mut self) -> FuelResult<Vec<String>> {
        let chain_config_json = to_value(&self.config.chain_conf)?;

        self.write_temp_config_file(chain_config_json)?;

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

        args.extend(vec![
            "--db-type".to_string(),
            match self.config.database_type {
                DbType::InMemory => "in-memory",
                DbType::RocksDb => "rocks-db",
            }
            .to_string(),
        ]);

        if let DbType::RocksDb = self.config.database_type {
            let path = if self.config.database_path.as_os_str().is_empty() {
                PathBuf::from(std::env::var("HOME").expect("HOME env var missing")).join(".fuel/db")
            } else {
                self.config.database_path.clone()
            };
            args.extend(vec![
                "--db-path".to_string(),
                path.to_string_lossy().to_string(),
            ]);
        }

        if self.config.max_database_cache_size != DEFAULT_CACHE_SIZE {
            args.push("--max-database-cache-size".to_string());
            args.push(self.config.max_database_cache_size.to_string());
        }

        if self.config.utxo_validation {
            args.push("--utxo-validation".to_string());
        }

        if self.config.manual_blocks_enabled {
            args.push("--manual_blocks_enabled".to_string());
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

        if self.config.vm_backtrace {
            args.push("--vm-backtrace".to_string());
        }

        Ok(args)
    }

    pub fn write_temp_config_file(&mut self, config: Value) -> FuelResult<()> {
        writeln!(self.config_file, "{}", &config.to_string())?;
        Ok(())
    }
}

#[derive(Clone, Default, Debug)]
pub struct SharedState {
    pub config: Config,
}

pub struct ServerParams {
    extended_config: ExtendedConfig,
}

pub struct Task {
    pub running_node: Pin<Box<JoinHandle<()>>>,
}

#[async_trait::async_trait]
impl RunnableTask for Task {
    async fn run(&mut self, state: &mut StateWatcher) -> anyhow::Result<bool> {
        state.while_started().await?;
        Ok(false)
    }

    async fn shutdown(self) -> anyhow::Result<()> {
        Ok(())
    }
}

pub struct FuelNode {
    pub config: Config,
}

#[async_trait::async_trait]
impl RunnableService for FuelNode {
    const NAME: &'static str = "FuelService";
    type SharedData = SharedState;
    type Task = Task;
    type TaskParams = ServerParams;

    fn shared_data(&self) -> Self::SharedData {
        SharedState {
            config: self.config.clone(),
        }
    }

    async fn into_task(
        self,
        _state: &StateWatcher,
        params: Self::TaskParams,
    ) -> anyhow::Result<Self::Task> {
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

        let address = extended_config.config.addr.clone();
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

        Ok(Task {
            running_node: Box::pin(join_handle),
        })
    }
}

pub struct FuelService {
    pub bound_address: SocketAddr,
    pub shared: SharedState,
    runner: ServiceRunner<FuelNode>,
}

impl FuelService {
    pub fn new(config: Config) -> FuelResult<Self> {
        let requested_port = config.addr.port();

        let bound_address = match requested_port {
            0 => get_socket_address(),
            _ if is_free(requested_port) => config.addr,
            _ => return Err(Error::IOError(std::io::ErrorKind::AddrInUse.into()).into()),
        };

        let config = Config {
            addr: bound_address.clone(),
            ..config
        };

        let runner = ServiceRunner::new_with_params(
            FuelNode {
                config: config.clone(),
            },
            ServerParams {
                extended_config: ExtendedConfig {
                    config,
                    config_file: NamedTempFile::new()?,
                },
            },
        );

        let shared = runner.shared.clone();

        Ok(FuelService {
            bound_address,
            shared,
            runner,
        })
    }

    pub async fn new_node(config: Config) -> FuelResult<Self> {
        let service = Self::new(config)?;

        service
            .runner
            .start_and_await()
            .await
            .map_err(|err| error!(InfrastructureError, "{err}"))?;
        Ok(service)
    }
}

#[async_trait::async_trait]
impl ServiceTrait for FuelService {
    fn start(&self) -> anyhow::Result<()> {
        self.runner.start()
    }

    async fn start_and_await(&self) -> anyhow::Result<State> {
        self.runner.start_and_await().await
    }

    async fn await_start_or_stop(&self) -> anyhow::Result<State> {
        self.runner.await_start_or_stop().await
    }

    fn stop(&self) -> bool {
        self.runner.stop()
    }

    async fn stop_and_await(&self) -> anyhow::Result<State> {
        self.runner.stop_and_await().await
    }

    async fn await_stop(&self) -> anyhow::Result<State> {
        self.runner.await_stop().await
    }

    fn state(&self) -> State {
        self.runner.state()
    }

    fn state_watcher(&self) -> StateWatcher {
        self.runner.state_watcher()
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
