use fuels_core::types::errors::{Error, Result as FuelResult};
use portpicker::is_free;
use std::net::SocketAddr;
use std::thread::JoinHandle;

use fuel_core_services::RunnableService;
use fuel_core_services::ServiceRunner;
use fuel_core_services::State;
use fuel_core_services::StateWatcher;
use fuel_core_services::{RunnableTask, Service as ServiceTrait};

use crate::node;
use crate::node::Config;

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
    // pub shared: SharedState,
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

        Ok(FuelService { bound_address })
    }

    pub fn new() -> FuelResult<Self> {
        let runner = ServiceRunner::new(task);
        let shared = runner.shared.clone();
        let bound_address = runner.shared.graph_ql.bound_address;
        Ok(FuelService {
            bound_address,
            shared,
            runner,
        })
    }
}

pub type Shared<T> = std::sync::Arc<T>;
use tokio::sync::watch;

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
