use fuels_core::types::errors::{Error, Result as FuelResult};
use portpicker::is_free;
use std::net::SocketAddr;
use tokio::task::JoinHandle;

use fuel_core_services::RunnableService;
use fuel_core_services::ServiceRunner;
use fuel_core_services::State;
use fuel_core_services::StateWatcher;
use fuel_core_services::{RunnableTask, Service as ServiceTrait};

use fuel_core_client::client::FuelClient;
use std::process::Stdio;
use tokio::process::Command;

pub const DEFAULT_CACHE_SIZE: usize = 10 * 1024 * 1024;


use fuels_core::{
    constants::WORD_SIZE,
    types::{coin::Coin, message::Message},
};

use crate::node::{Trigger, DbType, server_health_check, write_temp_config_file, get_node_config_json, ChainConfig, get_socket_address, Config};
use std::path::PathBuf;
use std::sync::Arc;

pub type SubServices = Vec<Box<dyn ServiceTrait + Send + Sync + 'static>>;

#[derive(Clone)]
pub struct SharedState {
    pub config: Config,
}

pub struct Task {
    /// The address bound by the system for serving the API
    pub shared: SharedState,
}

#[async_trait::async_trait]
impl RunnableService for Task {
    const NAME: &'static str = "FuelService";
    type SharedData = SharedState;
    type Task = Task;
    type TaskParams = ();

    fn shared_data(&self) -> Self::SharedData {
        self.shared.clone()
    }

    async fn into_task(self, _: &StateWatcher, _: Self::TaskParams) -> anyhow::Result<Self::Task> {
        Ok(self)
    }
}

#[async_trait::async_trait]
impl RunnableTask for Task {
    async fn run(&mut self, _: &mut StateWatcher) -> anyhow::Result<bool> {
        // The `axum::Server` has its internal loop. If `await` is finished, we get an internal
        // error or stop signal.
        Ok(false /* should_continue */)
    }

    async fn shutdown(self) -> anyhow::Result<()> {
        // Nothing to shut down because we don't have any temporary state that should be dumped,
        // and we don't spawn any sub-tasks that we need to finish or await.
        // The `axum::Server` was already gracefully shutdown at this point.
        Ok(())
    }
}

pub struct FuelService {
    // runner: ServiceRunner<Task>,
    pub bound_address: SocketAddr,
    // runner: ServiceRunner<Task>,
    // pub shared: SharedState,
    pub state: Shared<watch::Sender<State>>,
    
}

impl FuelService {
    pub async fn new_node(config: Config) -> FuelResult<Self> {
        let requested_port = config.addr.port();

        let bound_address = if requested_port == 0 {
            get_socket_address()
        } else if is_free(requested_port) {
            config.addr
        } else {
            return Err(Error::IOError(std::io::ErrorKind::AddrInUse.into()));
        };

        let join_handle = new_fuel_node(
            vec![],
            vec![],
            Config {
                addr: bound_address,
                ..config
            },
            None,
        )
        .await?;

        // Todo fix this
        let (sender, _) = watch::channel(State::Started);
        let state = Shared::new(sender);

        Ok(FuelService { bound_address, state })
    }

    // pub fn new() -> FuelResult<Self> {
    //     // let runner = ServiceRunner::new(task);
    //     let shared = runner.shared.clone();
    //     let bound_address = runner.shared.graph_ql.bound_address;
    //     Ok(FuelService {
    //         bound_address,
    //         // shared,
    //         // runner,
    //     })
    // }
}

pub type Shared<T> = std::sync::Arc<T>;
use tokio::sync::watch;

#[async_trait::async_trait]
impl ServiceTrait for FuelService {
    fn start(&self) -> anyhow::Result<()> {
        self.runner.start()
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


pub async fn new_fuel_node(
    coins: Vec<Coin>,
    messages: Vec<Message>,
    config: Config,
    chain_config: Option<ChainConfig>,
) -> FuelResult<JoinHandle<()>> {

    let (sender, _) = watch::channel(State::NotStarted);
    let state = Shared::new(sender);
    let stop_sender = state.clone();

    let config_json = get_node_config_json(coins, messages, chain_config);

    let temp_config_file = write_temp_config_file(config_json);

    let port = config.addr.port().to_string();
    let mut args = vec![
        "run".to_string(), // `fuel-core` is now run with `fuel-core run`
        "--ip".to_string(),
        "127.0.0.1".to_string(),
        "--port".to_string(),
        port,
        "--chain".to_string(),
        temp_config_file.path().to_str().unwrap().to_string(),
    ];

    args.extend(vec![
        "--db-type".to_string(),
        match config.database_type {
            DbType::InMemory => "in-memory",
            DbType::RocksDb => "rocks-db",
        }
            .to_string(),
    ]);

    if let DbType::RocksDb = config.database_type {
        let path = if config.database_path.as_os_str().is_empty() {
            PathBuf::from(std::env::var("HOME").expect("HOME env var missing")).join(".fuel/db")
        } else {
            config.database_path.clone()
        };
        args.extend(vec![
            "--db-path".to_string(),
            path.to_string_lossy().to_string(),
        ]);
    }

    if config.max_database_cache_size != DEFAULT_CACHE_SIZE {
        args.push("--max-database-cache-size".to_string());
        args.push(config.max_database_cache_size.to_string());
    }

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

    Ok(run_node(config, args, path).await?)
}

async fn run_node(config: Config, mut args: Vec<String>, path: &PathBuf) -> Result<JoinHandle<()>, Error> {
    let mut command = Command::new(path);

    command.stdin(Stdio::null());
    if config.silent {
        command.stdout(Stdio::null()).stderr(Stdio::null());
    }

    let running_node = command.args(args).kill_on_drop(true).env_clear().output();

    let client = FuelClient::from(config.addr);
    server_health_check(&client).await?;

    let join_handle = tokio::spawn(async move {
        let result = running_node
            .await
            .expect("error: Couldn't find fuel-core in PATH.");
        let stdout = String::from_utf8_lossy(&result.stdout);
        let stderr = String::from_utf8_lossy(&result.stderr);
        eprintln!("the exit status from the fuel binary was: {result:?}, stdout: {stdout}, stderr: {stderr}");
    });

    Ok(join_handle)
}

#[cfg(test)]
mod test {
    use crate::fuel_service::{
        Config,
        Task
    };

    #[test]
    fn test_expand_fn_simple() -> fuels_core::types::errors::Result<()> {


        Ok(())
    }
}
