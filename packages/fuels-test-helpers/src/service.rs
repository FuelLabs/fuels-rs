use std::net::SocketAddr;

#[cfg(feature = "fuel-core-lib")]
use fuel_core::service::{Config as ServiceConfig, FuelService as CoreFuelService};
use fuel_core_chain_config::{ChainConfig, StateConfig};
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
        let result = self.service.send_stop_signal_and_await_shutdown().await;

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
        use std::time::Duration;

        use fuel_core::{
            combined_database::CombinedDatabaseConfig,
            fuel_core_graphql_api::ServiceConfig as GraphQLConfig,
        };
        use fuel_core_chain_config::SnapshotReader;

        #[cfg(feature = "rocksdb")]
        use fuel_core::state::rocks_db::{ColumnsPolicy, DatabaseConfig};

        use crate::DbType;

        let snapshot_reader = SnapshotReader::new_in_memory(chain_config, state_config);

        let combined_db_config = CombinedDatabaseConfig {
            database_path: match &node_config.database_type {
                DbType::InMemory => Default::default(),
                DbType::RocksDb(path) => path.clone().unwrap_or_default(),
            },
            database_type: node_config.database_type.into(),
            #[cfg(feature = "rocksdb")]
            database_config: DatabaseConfig {
                cache_capacity: node_config.max_database_cache_size,
                max_fds: 512,
                columns_policy: ColumnsPolicy::Lazy,
            },
            #[cfg(feature = "rocksdb")]
            state_rewind_policy: Default::default(),
        };
        ServiceConfig {
            graphql_config: GraphQLConfig {
                addr: node_config.addr,
                max_queries_depth: 16,
                max_queries_complexity: 80000,
                max_queries_recursive_depth: 16,
                max_queries_resolver_recursive_depth: 1,
                max_queries_directives: 10,
                max_concurrent_queries: 1024,
                request_body_bytes_limit: 16 * 1024 * 1024,
                query_log_threshold_time: Duration::from_secs(2),
                api_request_timeout: Duration::from_secs(60),
                database_batch_size: 100,
                costs: Default::default(),
                number_of_threads: 2,
            },
            combined_db_config,
            snapshot_reader,
            utxo_validation: node_config.utxo_validation,
            debug: node_config.debug,
            block_production: node_config.block_production.into(),
            starting_exec_gas_price: node_config.starting_gas_price,
            ..ServiceConfig::local_node()
        }
    }
}
