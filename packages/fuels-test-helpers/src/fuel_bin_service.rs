use std::{
    net::{IpAddr, SocketAddr},
    time::Duration,
};

use fuel_core_client::client::FuelClient;
use fuel_core_services::State;
use fuels_core::{
    error,
    types::errors::{Error, Result as FuelResult},
};
use portpicker::{is_free, pick_unused_port};
use tokio::{process::Command, spawn, task::JoinHandle, time::sleep};

use crate::{node_types::NodeConfig, ExtendedConfig};

pub struct FuelService {
    pub bound_address: SocketAddr,
    handle: JoinHandle<()>,
}

impl FuelService {
    pub async fn new_node(config: ExtendedConfig) -> FuelResult<Self> {
        let requested_port = config.node_config.addr.port();

        let bound_address = match requested_port {
            0 => get_socket_address()?,
            _ if is_free(requested_port) => config.node_config.addr,
            _ => return Err(Error::IO(std::io::ErrorKind::AddrInUse.into())),
        };

        let node_config = NodeConfig {
            addr: bound_address,
            ..config.node_config
        };

        let extended_config = ExtendedConfig {
            node_config,
            ..config
        };

        let addr = extended_config.node_config.addr;
        let handle = run_node(extended_config).await?;
        server_health_check(addr).await?;

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

fn get_socket_address() -> FuelResult<SocketAddr> {
    let free_port = pick_unused_port().ok_or(error!(Other, "could not pick a free port"))?;
    let address: IpAddr = "127.0.0.1".parse().expect("is valid ip");

    Ok(SocketAddr::new(address, free_port))
}

async fn run_node(extended_config: ExtendedConfig) -> FuelResult<JoinHandle<()>> {
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

    let mut command = Command::new(path);
    let running_node = command.args(args).kill_on_drop(true).output();

    let join_handle = spawn(async move {
        // ensure drop is not called on the tmp dir and it lives throughout the lifetime of the node
        let _unused = tempdir;
        let result = running_node
            .await
            .expect("error: could not find `fuel-core` in PATH`");
        let stdout = String::from_utf8_lossy(&result.stdout);
        let stderr = String::from_utf8_lossy(&result.stderr);
        eprintln!("the exit status from the fuel binary was: {result:?}, stdout: {stdout}, stderr: {stderr}");
    });

    Ok(join_handle)
}
