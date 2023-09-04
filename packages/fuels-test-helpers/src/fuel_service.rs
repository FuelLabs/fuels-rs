use std::net::SocketAddr;
use fuels_core::types::errors::{Error, Result as FuelResult};
use portpicker::is_free;
use tokio::task::JoinHandle;

use fuel_core_services::Service as ServiceTrait;
use fuel_core_services::State;
use fuel_core_services::StateWatcher;
use fuel_core_services::ServiceRunner;
use fuel_core_services::RunnableService;

use crate::node::Config;
use crate::node;

pub type SubServices = Vec<Box<dyn ServiceTrait + Send + Sync + 'static>>;

#[derive(Clone)]
pub struct SharedState {
    // The transaction pool shared state.
    // pub txpool: fuel_core_txpool::service::SharedState<P2PAdapter, Database>,
    // The P2P network shared state.
    // #[cfg(feature = "p2p")]
    // pub network: Option<fuel_core_p2p::service::SharedState>,
    // #[cfg(feature = "relayer")]
    // The Relayer shared state.
    // pub relayer: Option<fuel_core_relayer::SharedState<Database>>,
    // The GraphQL shared state.
    // pub graph_ql: crate::fuel_core_graphql_api::service::SharedState,
    // The underlying database.
    // pub database: Database,
    // Subscribe to new block production.
    // pub block_importer: BlockImporterAdapter,
    // The config of the service.
    pub config: Config,
}

pub struct Task {
    /// The list of started sub services.
    services: SubServices,
    // The address bound by the system for serving the API
    pub shared: SharedState,
}

impl Task {
    /// Private inner method for initializing the fuel service task
    pub fn new(config: Config) -> anyhow::Result<Task> {
        let services =
        Ok(Task { services, shared: SharedState { config } })
    }

    #[cfg(test)]
    pub fn sub_services(&mut self) -> &mut SubServices {
        &mut self.services
    }
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

    async fn into_task(
        self,
        _: &StateWatcher,
        _: Self::TaskParams,
    ) -> anyhow::Result<Self::Task> {
        for service in &self.services {
            service.start_and_await().await?;
        }
        Ok(self)
    }
}

pub struct FuelService {
    pub bound_address: SocketAddr,
    // pub runner: JoinHandle<()>
    runner: ServiceRunner<Task>,
}

impl FuelService {
    pub async fn new_node(config: Config) -> FuelResult<Self> {
        let requested_port = config.addr.port();

        let bound_address = if requested_port == 0 {
            node::get_socket_address()
        } else if is_free(requested_port) {
            config.addr
        } else {
            return Err(Error::IOError(std::io::ErrorKind::AddrInUse.into()));
        };

        let join_handle = node::new_fuel_node(
            vec![],
            vec![],
            Config {
                addr: bound_address,
                ..config
            },
            None,
        )
        .await?;

        Ok(FuelService { bound_address, runner: ServiceRunner::new(join_handle) })
    }
}

#[async_trait::async_trait]
impl ServiceTrait for FuelService {
    fn start(&self) -> anyhow::Result<()> {
        unimplemented!()
        // self.runner.start()
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
        self.join_handle.abort()
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
