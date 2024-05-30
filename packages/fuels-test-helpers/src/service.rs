use std::net::SocketAddr;

#[cfg(feature = "fuel-core-lib")]
use fuel_core::service::{Config as ServiceConfig, FuelService as CoreFuelService};
use fuel_core_chain_config::{ChainConfig, StateConfig};
#[cfg(feature = "fuel-core-lib")]
use fuel_core_services::Service;
use fuel_core_services::State;
use fuels_core::types::errors::{error, Result};

#[cfg(not(feature = "fuel-core-lib"))]
use crate::fuel_bin_service::FuelService as BinFuelService;
use crate::NodeConfig;

pub struct FuelService {
    #[cfg(feature = "fuel-core-lib")]
    service: CoreFuelService,
    #[cfg(not(feature = "fuel-core-lib"))]
    service: BinFuelService,
    bound_address: SocketAddr,
}

impl FuelService {
    pub async fn start(
        node_config: NodeConfig,
        chain_config: ChainConfig,
        state_config: StateConfig,
    ) -> Result<Self> {
        #[cfg(feature = "fuel-core-lib")]
        let service = {
            let config = Self::service_config(node_config, chain_config, state_config);
            CoreFuelService::new_node(config)
                .await
                .map_err(|err| error!(Other, "{err}"))?
        };

        #[cfg(not(feature = "fuel-core-lib"))]
        let service = BinFuelService::new_node(node_config, chain_config, state_config).await?;

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

        result.map_err(|err| error!(Other, "{err}"))
    }

    pub fn bound_address(&self) -> SocketAddr {
        self.bound_address
    }

    #[cfg(feature = "fuel-core-lib")]
    fn service_config(
        node_config: NodeConfig,
        chain_config: ChainConfig,
        state_config: StateConfig,
    ) -> ServiceConfig {
        use fuel_core::combined_database::CombinedDatabaseConfig;
        use fuel_core_chain_config::SnapshotReader;

        use crate::{DbType, MAX_DATABASE_CACHE_SIZE};

        let snapshot_reader = SnapshotReader::new_in_memory(chain_config, state_config);

        let combined_db_config = CombinedDatabaseConfig {
            max_database_cache_size: node_config
                .max_database_cache_size
                .unwrap_or(MAX_DATABASE_CACHE_SIZE),
            database_path: match &node_config.database_type {
                DbType::InMemory => Default::default(),
                DbType::RocksDb(path) => path.clone().unwrap_or_default(),
            },
            database_type: node_config.database_type.into(),
        };
        ServiceConfig {
            addr: node_config.addr,
            combined_db_config,
            snapshot_reader,
            utxo_validation: node_config.utxo_validation,
            debug: node_config.debug,
            block_production: node_config.block_production.into(),
            static_gas_price: node_config.static_gas_price,
            ..ServiceConfig::local_node()
        }
    }
}
