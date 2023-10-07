#[cfg(feature = "fuel-core-lib")]
use fuel_core::service::FuelService as FService;

#[cfg(not(feature = "fuel-core-lib"))]
use crate::fuel_bin_service::FuelService as FService;

use fuel_core_services::{Service, State};

use crate::Config;
use fuels_core::types::errors::{error, Error, Result};
use std::net::SocketAddr;

pub struct FuelService {
    service: FService,
}

impl FuelService {
    pub async fn start(config: Config) -> Result<Self> {
        let service = FService::new_node(config.into())
            .await
            .map_err(|err| error!(InfrastructureError, "{err}"))?;

        Ok(FuelService { service })
    }

    pub async fn stop(&self) -> Result<State> {
        self.service
            .stop_and_await()
            .await
            .map_err(|err| error!(InfrastructureError, "{err}"))
    }

    pub fn bound_address(&self) -> SocketAddr {
        self.service.bound_address
    }

    pub fn state(&self) -> State {
        self.service.state()
    }
}
