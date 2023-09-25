use fuels_core::types::errors::{Error, Result as FuelResult};
use portpicker::is_free;
use std::net::SocketAddr;

use std::net::Ipv4Addr;
use std::pin::Pin;

use core::future::Future;
use fuel_core_services::RunnableService;
use fuel_core_services::ServiceRunner;
use fuel_core_services::State;
use fuel_core_services::StateWatcher;
pub use fuel_core_services::{RunnableTask, Service};

use fuel_core_client::client::FuelClient;
use futures::FutureExt;
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;
use tokio::task::JoinHandle;

pub const DEFAULT_CACHE_SIZE: usize = 10 * 1024 * 1024;

use fuels_core::types::{coin::Coin, message::Message};

use crate::node::{
    get_socket_address, new_fuel_node_arguments, server_health_check, ChainConfig, Config,
};

#[derive(Clone, Default, Debug)]
pub struct SharedState {
    pub config: Config,
}

pub struct ServerParams {
    config: Config,
}

// #[derive(Default)]
// pub struct ServerParams {
//     config: Config,
// }

pub struct Task {
    pub running_node: Pin<Box<JoinHandle<()>>>,
}
#[async_trait::async_trait]
impl RunnableTask for Task {
    async fn run(&mut self, _: &mut StateWatcher) -> anyhow::Result<bool> {
        self.running_node.as_mut().await?;
        // self.running_node.as_mut().await;
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

pub struct FuelNode {
    pub config: Config,
}

#[async_trait::async_trait]
impl RunnableService for FuelNode {
    const NAME: &'static str = "FuelNode";
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
        let ServerParams { mut config } = params;

        let (config, args, temp_file) = new_fuel_node_arguments(config)?;

        let file_path = args.get(6).unwrap();

        if std::fs::metadata(file_path).is_ok() {

            println!("File '{}' exists.", file_path);
        } else {
            println!("File '{}' does not exist.", file_path);
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
        let running_node = command
            .args(args)
            .kill_on_drop(true)
            .output();

        let address = config.addr.clone();
        let client = FuelClient::from(address);
        server_health_check(&client).await?;

        let join_handle = tokio::task::spawn(async move {
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

        let bound_address = if requested_port == 0 {
            get_socket_address()
        } else if is_free(requested_port) {
            config.addr
        } else {
            return Err(Error::IOError(std::io::ErrorKind::AddrInUse.into()).into());
        };

        let config = Config {
            addr: bound_address.clone(),
            ..config
        };

        let runner = ServiceRunner::new_with_params(
            FuelNode {
                config: config.clone(),
            },
            ServerParams { config },
        );
        let shared = runner.shared.clone();
        dbg!(bound_address);

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
            .map_err(|err| fuels_core::error!(InfrastructureError, "{err}"))?;
        Ok(service)
    }
}

pub type Shared<T> = std::sync::Arc<T>;

#[async_trait::async_trait]
impl Service for FuelService {
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
        unimplemented!()
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
