use fuels_core::types::errors::{Error, Result as FuelResult};
use portpicker::is_free;
use std::net::{IpAddr, SocketAddr};
use tokio::task::JoinHandle;

use std::net::Ipv4Addr;
use std::pin::Pin;

use core::future::Future;
use fuel_core_services::RunnableService;
use fuel_core_services::ServiceRunner;
use fuel_core_services::State;
use fuel_core_services::StateWatcher;
pub use fuel_core_services::{RunnableTask, Service};

use fuel_core_client::client::FuelClient;
use std::process::Stdio;
use tokio::process::Command;

pub const DEFAULT_CACHE_SIZE: usize = 10 * 1024 * 1024;

use fuels_core::{
    constants::WORD_SIZE,
    types::{coin::Coin, message::Message},
};

use crate::node::{
    get_node_config_json, get_socket_address, new_fuel_node_arguments, run_node,
    server_health_check, write_temp_config_file, ChainConfig, Config, DbType, Trigger,
};
use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::sync::Arc;

#[derive(Clone)]
pub struct SharedState {
    pub config: Config,
}

#[derive(Default)]
pub struct ServerParams {
    coins: Vec<Coin>,
    messages: Vec<Message>,
    config: Config,
    chain_config: Option<ChainConfig>,
}

pub struct FuelNode {
    pub running_node:
        Pin<Box<dyn Future<Output = std::io::Result<std::process::Output>> + Send + 'static>>,
    // pub join_handler: JoinHandle<()>,
    pub shared: SharedState,
}

impl FuelNode {
    pub fn new(
        coins: Vec<Coin>,
        messages: Vec<Message>,
        config: Config,
        chain_config: Option<ChainConfig>,
    ) -> FuelResult<Self> {
        let requested_port = config.addr.port();

        let bound_address = if requested_port == 0 {
            get_socket_address()
        } else if is_free(requested_port) {
            config.addr
        } else {
            return Err(Error::IOError(std::io::ErrorKind::AddrInUse.into()));
        };

        let (config, args, path) = new_fuel_node_arguments(
            coins,
            messages,
            Config {
                addr: bound_address,
                ..config
            },
            chain_config,
        )?;

        let mut command = Command::new(path);
        command.stdin(Stdio::null());
        if config.silent {
            command.stdout(Stdio::null()).stderr(Stdio::null());
        }
        let running_node = command.args(args).kill_on_drop(true).env_clear().output();

        Ok(Self {
            running_node: Box::pin(running_node),
            shared: SharedState { config },
        })
    }
}

#[async_trait::async_trait]
impl RunnableTask for FuelNode {
    async fn run(&mut self, _: &mut StateWatcher) -> anyhow::Result<bool> {
        Ok(false /* should_continue */)
    }

    async fn shutdown(self) -> anyhow::Result<()> {
        // Nothing to shut down because we don't have any temporary state that should be dumped,
        // and we don't spawn any sub-tasks that we need to finish or await.
        // The `axum::Server` was already gracefully shutdown at this point.
        Ok(())
    }
}

#[async_trait::async_trait]
impl RunnableService for FuelNode {
    const NAME: &'static str = "FuelNode";
    type SharedData = SharedState;
    type Task = FuelNode;
    type TaskParams = ServerParams;

    fn shared_data(&self) -> Self::SharedData {
        self.shared.clone()
    }

    async fn into_task(
        self,
        state: &StateWatcher,
        params: Self::TaskParams,
    ) -> anyhow::Result<Self::Task> {
        let ServerParams {
            coins,
            messages,
            config,
            chain_config,
        } = params;

        let (config, args, path) = new_fuel_node_arguments(coins, messages, config, chain_config)?;

        let mut command = Command::new(path);
        command.stdin(Stdio::null());
        if config.silent {
            command.stdout(Stdio::null()).stderr(Stdio::null());
        }
        let running_node = command.args(args).kill_on_drop(true).env_clear().output();

        Ok(Self {
            running_node: Box::pin(running_node),
            shared: SharedState { config },
        })
    }
}

pub struct FuelService {
    pub bound_address: SocketAddr,
    runner: ServiceRunner<FuelNode>,
}

impl FuelService {
    pub fn new(
        coins: Vec<Coin>,
        messages: Vec<Message>,
        config: Config,
        chain_config: Option<ChainConfig>,
    ) -> FuelResult<Self> {
        let fuel_node = FuelNode::new(coins, messages, config, chain_config)?;
        let runner = ServiceRunner::new(fuel_node);
        let bound_address = SocketAddr::new(
            Ipv4Addr::new(127, 0, 0, 1).into(),
            runner.shared.config.addr.port(),
        );
        Ok(FuelService {
            bound_address,
            runner,
        })
    }

    pub async fn new_node(config: Config) -> FuelResult<Self> {
        let (config, args, path) = new_fuel_node_arguments(
            vec![],
            vec![],
            Config {
                addr: bound_address,
                ..config
            },
            None,
        )?;

        Ok(FuelService::new(config, args, &path)?)

        // Ok(FuelService {
        //     bound_address,
        //     runner: unimplemented!(),
        // })
    }
}

pub type Shared<T> = std::sync::Arc<T>;
use tokio::sync::watch;

#[async_trait::async_trait]
impl Service for FuelService {
    fn start(&self) -> anyhow::Result<()> {
        unimplemented!()
    }

    async fn start_and_await(&self) -> anyhow::Result<State> {
        unimplemented!()
        // self.runner.start_and_await().await
    }

    async fn await_start_or_stop(&self) -> anyhow::Result<State> {
        unimplemented!()
        // self.runner.await_start_or_stop().await
    }

    fn stop(&self) -> bool {
        unimplemented!()
    }

    async fn stop_and_await(&self) -> anyhow::Result<State> {
        unimplemented!()
        // self.runner.stop_and_await().await
    }

    async fn await_stop(&self) -> anyhow::Result<State> {
        unimplemented!()
        // self.runner.await_stop().await
    }

    fn state(&self) -> State {
        unimplemented!()
        // self.runner.state()
    }

    fn state_watcher(&self) -> StateWatcher {
        unimplemented!()
        // self.runner.state_watcher()
    }
}

// pub async fn new_fuel_node(
//     coins: Vec<Coin>,
//     messages: Vec<Message>,
//     config: Config,
//     chain_config: Option<ChainConfig>,
// ) -> FuelResult<JoinHandle<()>> {
//     let (sender, _) = watch::channel(State::NotStarted);
//     let state = Shared::new(sender);
//     let stop_sender = state.clone();
//
//     let config_json = get_node_config_json(coins, messages, chain_config);
//
//     let temp_config_file = write_temp_config_file(config_json);
//
//     let port = config.addr.port().to_string();
//     let mut args = vec![
//         "run".to_string(), // `fuel-core` is now run with `fuel-core run`
//         "--ip".to_string(),
//         "127.0.0.1".to_string(),
//         "--port".to_string(),
//         port,
//         "--chain".to_string(),
//         temp_config_file.path().to_str().unwrap().to_string(),
//     ];
//
//     args.extend(vec![
//         "--db-type".to_string(),
//         match config.database_type {
//             DbType::InMemory => "in-memory",
//             DbType::RocksDb => "rocks-db",
//         }
//         .to_string(),
//     ]);
//
//     if let DbType::RocksDb = config.database_type {
//         let path = if config.database_path.as_os_str().is_empty() {
//             PathBuf::from(std::env::var("HOME").expect("HOME env var missing")).join(".fuel/db")
//         } else {
//             config.database_path.clone()
//         };
//         args.extend(vec![
//             "--db-path".to_string(),
//             path.to_string_lossy().to_string(),
//         ]);
//     }
//
//     if config.max_database_cache_size != DEFAULT_CACHE_SIZE {
//         args.push("--max-database-cache-size".to_string());
//         args.push(config.max_database_cache_size.to_string());
//     }
//
//     if config.utxo_validation {
//         args.push("--utxo-validation".to_string());
//     }
//
//     if config.manual_blocks_enabled {
//         args.push("--manual_blocks_enabled".to_string());
//     }
//
//     match config.block_production {
//         Trigger::Instant => {
//             args.push("--poa-instant=true".to_string());
//         }
//         Trigger::Never => {
//             args.push("--poa-instant=false".to_string());
//         }
//         Trigger::Interval { block_time } => {
//             args.push(format!(
//                 "--poa-interval-period={}ms",
//                 block_time.as_millis()
//             ));
//         }
//         Trigger::Hybrid {
//             min_block_time,
//             max_tx_idle_time,
//             max_block_time,
//         } => {
//             args.push(format!(
//                 "--poa-hybrid-min-time={}ms",
//                 min_block_time.as_millis()
//             ));
//             args.push(format!(
//                 "--poa-hybrid-idle-time={}ms",
//                 max_tx_idle_time.as_millis()
//             ));
//             args.push(format!(
//                 "--poa-hybrid-max-time={}ms",
//                 max_block_time.as_millis()
//             ));
//         }
//     };
//
//     if config.vm_backtrace {
//         args.push("--vm-backtrace".to_string());
//     }
//
//     // Warn if there is more than one binary in PATH.
//     let binary_name = "fuel-core";
//     let paths = which::which_all(binary_name)
//         .unwrap_or_else(|_| panic!("failed to list '{binary_name}' binaries"))
//         .collect::<Vec<_>>();
//     let path = paths
//         .first()
//         .unwrap_or_else(|| panic!("no '{binary_name}' in PATH"));
//     if paths.len() > 1 {
//         eprintln!(
//             "found more than one '{}' binary in PATH, using '{}'",
//             binary_name,
//             path.display()
//         );
//     }
//
//     Ok(run_node(config, args, path).await?)
// }
//
// async fn run_node(
//     config: Config,
//     args: Vec<String>,
//     path: &PathBuf,
// ) -> Result<JoinHandle<()>, Error> {
//     let mut command = Command::new(path);
//
//     command.stdin(Stdio::null());
//     if config.silent {
//         command.stdout(Stdio::null()).stderr(Stdio::null());
//     }
//
//     let running_node = command.args(args).kill_on_drop(true).env_clear().output();
//
//     let client = FuelClient::from(config.addr);
//     server_health_check(&client).await?;
//
//     let join_handle = tokio::task::spawn(async move {
//         let result = running_node
//             .await
//             .expect("error: Couldn't find fuel-core in PATH.");
//         let stdout = String::from_utf8_lossy(&result.stdout);
//         let stderr = String::from_utf8_lossy(&result.stderr);
//         eprintln!("the exit status from the fuel binary was: {result:?}, stdout: {stdout}, stderr: {stderr}");
//     });
//
//     Ok(join_handle)
// }
//
// pub async fn new_fuel_node_task(
//     coins: Vec<Coin>,
//     messages: Vec<Message>,
//     config: Config,
//     chain_config: Option<ChainConfig>,
// ) -> FuelResult<(Shared<watch::Sender<State>>, JoinHandle<()>)> {
//     let (sender, _) = watch::channel(State::NotStarted);
//     let state = Shared::new(sender);
//     let stop_sender = state.clone();
//
//     let config_json = get_node_config_json(coins, messages, chain_config);
//
//     let temp_config_file = write_temp_config_file(config_json);
//
//     let port = config.addr.port().to_string();
//     let mut args = vec![
//         "run".to_string(), // `fuel-core` is now run with `fuel-core run`
//         "--ip".to_string(),
//         "127.0.0.1".to_string(),
//         "--port".to_string(),
//         port,
//         "--chain".to_string(),
//         temp_config_file.path().to_str().unwrap().to_string(),
//     ];
//
//     args.extend(vec![
//         "--db-type".to_string(),
//         match config.database_type {
//             DbType::InMemory => "in-memory",
//             DbType::RocksDb => "rocks-db",
//         }
//         .to_string(),
//     ]);
//
//     if let DbType::RocksDb = config.database_type {
//         let path = if config.database_path.as_os_str().is_empty() {
//             PathBuf::from(std::env::var("HOME").expect("HOME env var missing")).join(".fuel/db")
//         } else {
//             config.database_path.clone()
//         };
//         args.extend(vec![
//             "--db-path".to_string(),
//             path.to_string_lossy().to_string(),
//         ]);
//     }
//
//     if config.max_database_cache_size != DEFAULT_CACHE_SIZE {
//         args.push("--max-database-cache-size".to_string());
//         args.push(config.max_database_cache_size.to_string());
//     }
//
//     if config.utxo_validation {
//         args.push("--utxo-validation".to_string());
//     }
//
//     if config.manual_blocks_enabled {
//         args.push("--manual_blocks_enabled".to_string());
//     }
//
//     match config.block_production {
//         Trigger::Instant => {
//             args.push("--poa-instant=true".to_string());
//         }
//         Trigger::Never => {
//             args.push("--poa-instant=false".to_string());
//         }
//         Trigger::Interval { block_time } => {
//             args.push(format!(
//                 "--poa-interval-period={}ms",
//                 block_time.as_millis()
//             ));
//         }
//         Trigger::Hybrid {
//             min_block_time,
//             max_tx_idle_time,
//             max_block_time,
//         } => {
//             args.push(format!(
//                 "--poa-hybrid-min-time={}ms",
//                 min_block_time.as_millis()
//             ));
//             args.push(format!(
//                 "--poa-hybrid-idle-time={}ms",
//                 max_tx_idle_time.as_millis()
//             ));
//             args.push(format!(
//                 "--poa-hybrid-max-time={}ms",
//                 max_block_time.as_millis()
//             ));
//         }
//     };
//
//     if config.vm_backtrace {
//         args.push("--vm-backtrace".to_string());
//     }
//
//     // Warn if there is more than one binary in PATH.
//     let binary_name = "fuel-core";
//     let paths = which::which_all(binary_name)
//         .unwrap_or_else(|_| panic!("failed to list '{binary_name}' binaries"))
//         .collect::<Vec<_>>();
//     let path = paths
//         .first()
//         .unwrap_or_else(|| panic!("no '{binary_name}' in PATH"));
//     if paths.len() > 1 {
//         eprintln!(
//             "found more than one '{}' binary in PATH, using '{}'",
//             binary_name,
//             path.display()
//         );
//     }
//
//     let (state, join_handle) = run_node_task(config, args, path).await?;
//     Ok((state, join_handle))
// }
//
// async fn run_node_task(
//     config: Config,
//     args: Vec<String>,
//     path: &PathBuf,
// ) -> Result<(Shared<watch::Sender<State>>, JoinHandle<()>), Error> {
//     let (sender, _) = watch::channel(State::NotStarted);
//     let state = Shared::new(sender);
//     let stop_sender = state.clone();
//
//     let mut command = Command::new(path);
//
//     command.stdin(Stdio::null());
//     if config.silent {
//         command.stdout(Stdio::null()).stderr(Stdio::null());
//     }
//
//     let running_node = command.args(args).kill_on_drop(true).env_clear().output();
//
//     let client = FuelClient::from(config.addr);
//     server_health_check(&client).await?;
//
//     let join_handle = tokio::task::spawn(async move {
//         let mut state: StateWatcher = stop_sender.subscribe().into();
//
//         if state.borrow_and_update().not_started() {
//             state.changed().await.expect("The service is destroyed");
//         }
//
//         if !state.borrow().starting() {
//             return;
//         }
//
//         let result = running_node
//             .await
//             .expect("error: Couldn't find fuel-core in PATH.");
//         let stdout = String::from_utf8_lossy(&result.stdout);
//         let stderr = String::from_utf8_lossy(&result.stderr);
//         eprintln!("the exit status from the fuel binary was: {result:?}, stdout: {stdout}, stderr: {stderr}");
//
//         stop_sender.send_if_modified(|s| {
//             if s.starting() {
//                 *s = State::Started;
//                 true
//             } else {
//                 false
//             }
//         });
//     });
//
//     Ok((state, join_handle))
// }
//
