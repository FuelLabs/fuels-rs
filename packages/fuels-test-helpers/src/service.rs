use crate::Config;
use fuels_core::types::errors::{error, Error, Result};
use std::net::SocketAddr;

use fuel_core_services::State;

#[cfg(feature = "fuel-core-lib")]
use fuel_core::service::FuelService as CoreFuelService;
#[cfg(feature = "fuel-core-lib")]
use fuel_core_services::Service;

#[cfg(not(feature = "fuel-core-lib"))]
use crate::fuel_bin_service::FuelService as BinFuelService;

pub struct FuelService {
    #[cfg(feature = "fuel-core-lib")]
    service: CoreFuelService,
    #[cfg(not(feature = "fuel-core-lib"))]
    service: BinFuelService,
    bound_address: SocketAddr,
}

impl FuelService {
    pub async fn start(config: Config) -> Result<Self> {
        #[cfg(feature = "fuel-core-lib")]
        let service = CoreFuelService::new_node(config.into())
            .await
            .map_err(|err| error!(InfrastructureError, "{}", err))?;

        #[cfg(not(feature = "fuel-core-lib"))]
        let service = BinFuelService::new_node(config)
            .await
            .map_err(|err| error!(InfrastructureError, "{}", err))?;

        let bound_address = service.bound_address;

        Ok(FuelService {
            service,
            bound_address,
        })
    }

    pub async fn stop(&self) -> Result<State> {
        #[cfg(feature = "fuel-core-lib")]
        let result = self.service.stop_and_await().await;

        #[cfg(not(feature = "fuel-core-lib"))]
        let result = self.service.stop();

        result.map_err(|err| error!(InfrastructureError, "{}", err))
    }

    pub fn bound_address(&self) -> SocketAddr {
        self.bound_address
    }
}
