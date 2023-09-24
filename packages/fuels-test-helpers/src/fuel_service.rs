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

use std::process::Stdio;
use tokio::process::Command;
use fuel_core_client::client::FuelClient;

pub const DEFAULT_CACHE_SIZE: usize = 10 * 1024 * 1024;

use fuels_core::types::{coin::Coin, message::Message};

use crate::node::{
    get_socket_address, new_fuel_node_arguments, server_health_check, ChainConfig, Config,
};

#[derive(Clone)]
pub struct SharedState {
    pub config: Config,
}

#[derive(Default)]
pub struct ServerParams {
    config: Config,
}

pub struct FuelNode {
    pub running_node:
        Pin<Box<dyn Future<Output = std::io::Result<std::process::Output>> + Send + 'static>>,
    pub shared: SharedState,
}

impl FuelNode {
    pub fn new(config: Config) -> FuelResult<Self> {
        let requested_port = config.addr.port();

        let bound_address = if requested_port == 0 {
            get_socket_address()
        } else if is_free(requested_port) {
            config.addr
        } else {
            return Err(Error::IOError(std::io::ErrorKind::AddrInUse.into()));
        };

        let (config, args, path) = new_fuel_node_arguments(Config {
            addr: bound_address,
            ..config
        })?;

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
    async fn run(&mut self, state: &mut StateWatcher) -> anyhow::Result<bool> {
        let address = self.shared.config.addr;
        let client = FuelClient::from(address);
        server_health_check(&client).await?;

        let join_handle = tokio::task::spawn(async move {

            let result = self.running_node
                .await
                .as_mut()
                .expect("error: Couldn't find fuel-core in PATH.");
            let stdout = String::from_utf8_lossy(&result.stdout);
            let stderr = String::from_utf8_lossy(&result.stderr);
            eprintln!("the exit status from the fuel binary was: {result:?}, stdout: {stdout}, stderr: {stderr}");

        });


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
        _state: &StateWatcher,
        params: Self::TaskParams,
    ) -> anyhow::Result<Self::Task> {
        let ServerParams { config } = params;
        // TODO fix config
        let (config_, args, path) = new_fuel_node_arguments(config)?;

        let mut command = Command::new(path);
        command.stdin(Stdio::null());
        if config_.silent {
            command.stdout(Stdio::null()).stderr(Stdio::null());
        }
        let running_node = command.args(args).kill_on_drop(true).env_clear().output();

        Ok(Self {
            running_node: Box::pin(running_node),
            shared: SharedState { config: config_ },
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
        let fuel_node = FuelNode::new(config)?;
        let runner = ServiceRunner::new(fuel_node);
        let shared = runner.shared.clone();

        let bound_address = SocketAddr::new(
            Ipv4Addr::new(127, 0, 0, 1).into(),
            runner.shared.config.addr.port(),
        );
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
