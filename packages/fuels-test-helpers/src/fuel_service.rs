use std::net::SocketAddr;
use std::thread::JoinHandle;
use fuels_core::types::errors::{Error, Result as FuelResult};
use portpicker::is_free;

use fuel_core_services::Service as ServiceTrait;
use fuel_core_services::State;
use fuel_core_services::StateWatcher;
use fuel_core_services::ServiceRunner;
use fuel_core_services::RunnableService;

use crate::node::Config;
use crate::node;

pub type SubServices = Vec<Box<dyn ServiceTrait + Send + Sync + 'static>>;
pub struct GraphqlService {
    bound_address: SocketAddr,
}

#[derive(Clone)]
pub struct SharedState {
    pub config: Config,
}

pub struct Task {
    /// The list of started sub services.
    services: SubServices,
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

    async fn into_task(
        self,
        _: &StateWatcher,
        _: Self::TaskParams,
    ) -> anyhow::Result<Self::Task> {
        Ok(self)
    }
}

pub struct FuelService {
    pub bound_address: SocketAddr,
    // pub runner: JoinHandle<()>,
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

pub type Shared<T> = std::sync::Arc<T>;
use tokio::sync::watch;

// #[tracing::instrument(skip_all, fields(service = S::NAME))]
/// Initialize the background loop as a spawned task.
fn initialize_loop<S>(
    service: S,
    params: S::TaskParams,
    // metric: ServiceLifecycle,
) -> Shared<watch::Sender<State>>
    where
        S: RunnableService + 'static,
{
    let (sender, _) = watch::channel(State::NotStarted);
    let state = Shared::new(sender);
    let stop_sender = state.clone();
    // Spawned as a task to check if the service is already running and to capture any panics.
    tokio::task::spawn(
        async move {
        //     tracing::debug!("running");
        //     let run = std::panic::AssertUnwindSafe(run(
        //         service,
        //         stop_sender.clone(),
        //         params,
        //         metric,
        //     ));
        //     tracing::debug!("awaiting run");
        //     let result = run.catch_unwind().await;
        //
        //     let stopped_state = if let Err(e) = result {
        //         let panic_information = panic_to_string(e);
        //         State::StoppedWithError(panic_information)
        //     } else {
        //         State::Stopped
        //     };
        //
        //     tracing::debug!("shutting down {:?}", stopped_state);
        //
        //     let _ = stop_sender.send_if_modified(|state| {
        //         if !state.stopped() {
        //             *state = stopped_state.clone();
        //             tracing::debug!("Wasn't stopped, so sent stop.");
        //             true
        //         } else {
        //             tracing::debug!("Was already stopped.");
        //             false
        //         }
        //     });
        //
        //     if let State::StoppedWithError(err) = stopped_state {
        //         std::panic::resume_unwind(Box::new(err));
        //     }
        // }
            .in_current_span(),
    );
    state
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
