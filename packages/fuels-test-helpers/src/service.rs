use crate::Config;
use fuel_core_services::State;
use fuels_core::types::errors::{error, Error, Result};
use std::net::SocketAddr;

#[cfg(feature = "fuel-core-lib")]
use fuel_core_services::Service;

#[cfg(not(feature = "fuel-core-lib"))]
use crate::fuel_bin_service::FuelService as BinFuelService;

#[cfg(feature = "fuel-core-lib")]
use fuel_core::service::FuelService as CoreFuelService;

pub enum FuelService {
    #[cfg(feature = "fuel-core-lib")]
    Core {
        service: CoreFuelService,
        bound_address: SocketAddr,
    },

    #[cfg(not(feature = "fuel-core-lib"))]
    Bin {
        service: BinFuelService,
        bound_address: SocketAddr,
    },
}

impl FuelService {
    pub async fn start(config: Config) -> Result<Self> {
        Ok({
            #[cfg(not(feature = "fuel-core-lib"))]
            {
                let service = BinFuelService::new_node(config)
                    .await
                    .map_err(|err| error!(InfrastructureError, "{}", err))?;
                let bound_address = service.bound_address;
                FuelService::Bin {
                    service,
                    bound_address,
                }
            }

            #[cfg(feature = "fuel-core-lib")]
            {
                let service = CoreFuelService::new_node(config.into())
                    .await
                    .map_err(|err| error!(InfrastructureError, "{}", err))?;
                let bound_address = service.bound_address;
                FuelService::Core {
                    service,
                    bound_address,
                }
            }
        })
    }

    pub async fn stop(&self) -> Result<State> {
        match self {
            #[cfg(feature = "fuel-core-lib")]
            FuelService::Core { service, .. } => service.stop_and_await().await,
            #[cfg(not(feature = "fuel-core-lib"))]
            FuelService::Bin { service, .. } => service.stop().await,
        }
        .map_err(|err| error!(InfrastructureError, "{}", err))
    }

    pub fn bound_address(&self) -> SocketAddr {
        match self {
            #[cfg(feature = "fuel-core-lib")]
            FuelService::Core { bound_address, .. } => *bound_address,
            #[cfg(not(feature = "fuel-core-lib"))]
            FuelService::Bin { bound_address, .. } => *bound_address,
        }
    }
}
