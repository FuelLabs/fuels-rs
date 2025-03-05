use std::{sync::Arc, time::Duration};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use fuel_core_client::client::types::NodeInfo;
use fuel_tx::ConsensusParameters;
use fuels_core::types::errors::Result;
use tokio::sync::RwLock;

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait CacheableRpcs {
    async fn consensus_parameters(&self) -> Result<ConsensusParameters>;
    async fn node_info(&self) -> Result<NodeInfo>;
}

trait Clock {
    fn now(&self) -> DateTime<Utc>;
}

#[derive(Debug, Clone)]
pub struct TtlConfig {
    pub consensus_parameters: Duration,
}

impl Default for TtlConfig {
    fn default() -> Self {
        TtlConfig {
            consensus_parameters: Duration::from_secs(60),
        }
    }
}

#[derive(Debug, Clone)]
struct Dated<T> {
    value: T,
    date: DateTime<Utc>,
}

impl<T> Dated<T> {
    fn is_stale(&self, now: DateTime<Utc>, ttl: Duration) -> bool {
        self.date + ttl < now
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SystemClock;
impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

#[derive(Debug, Clone)]
pub struct CachedClient<Client, Clock = SystemClock> {
    client: Client,
    ttl_config: TtlConfig,
    cached_consensus_params: Arc<RwLock<Option<Dated<ConsensusParameters>>>>,
    cached_node_info: Arc<RwLock<Option<Dated<NodeInfo>>>>,
    clock: Clock,
}

impl<Client, Clock> CachedClient<Client, Clock> {
    pub fn new(client: Client, ttl: TtlConfig, clock: Clock) -> Self {
        Self {
            client,
            ttl_config: ttl,
            cached_consensus_params: Default::default(),
            cached_node_info: Default::default(),
            clock,
        }
    }

    pub fn set_ttl(&mut self, ttl: TtlConfig) {
        self.ttl_config = ttl
    }

    pub fn inner(&self) -> &Client {
        &self.client
    }

    pub fn inner_mut(&mut self) -> &mut Client {
        &mut self.client
    }
}

impl<Client, Clk> CachedClient<Client, Clk>
where
    Client: CacheableRpcs,
{
    pub async fn clear(&self) {
        *self.cached_consensus_params.write().await = None;
    }
}

#[async_trait]
impl<Client, Clk> CacheableRpcs for CachedClient<Client, Clk>
where
    Clk: Clock + Send + Sync,
    Client: CacheableRpcs + Send + Sync,
{
    async fn consensus_parameters(&self) -> Result<ConsensusParameters> {
        {
            let read_lock = self.cached_consensus_params.read().await;
            if let Some(entry) = read_lock.as_ref() {
                if !entry.is_stale(self.clock.now(), self.ttl_config.consensus_parameters) {
                    return Ok(entry.value.clone());
                }
            }
        }

        let mut write_lock = self.cached_consensus_params.write().await;

        // because it could have been updated since we last checked
        if let Some(entry) = write_lock.as_ref() {
            if !entry.is_stale(self.clock.now(), self.ttl_config.consensus_parameters) {
                return Ok(entry.value.clone());
            }
        }

        let fresh_parameters = self.client.consensus_parameters().await?;
        *write_lock = Some(Dated {
            value: fresh_parameters.clone(),
            date: self.clock.now(),
        });

        Ok(fresh_parameters)
    }

    async fn node_info(&self) -> Result<NodeInfo> {
        // must borrow from consensus_parameters to keep the change non-breaking
        let ttl = self.ttl_config.consensus_parameters;
        {
            let read_lock = self.cached_node_info.read().await;
            if let Some(entry) = read_lock.as_ref() {
                if !entry.is_stale(self.clock.now(), ttl) {
                    return Ok(entry.value.clone());
                }
            }
        }

        let mut write_lock = self.cached_node_info.write().await;

        // because it could have been updated since we last checked
        if let Some(entry) = write_lock.as_ref() {
            if !entry.is_stale(self.clock.now(), ttl) {
                return Ok(entry.value.clone());
            }
        }

        let fresh_node_info = self.client.node_info().await?;
        *write_lock = Some(Dated {
            value: fresh_node_info.clone(),
            date: self.clock.now(),
        });

        Ok(fresh_node_info)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use fuel_core_client::client::schema::{
        node_info::{IndexationFlags, TxPoolStats},
        U64,
    };
    use fuel_types::ChainId;

    use super::*;

    #[derive(Clone, Default)]
    struct TestClock {
        time: Arc<Mutex<DateTime<Utc>>>,
    }

    impl TestClock {
        fn update_time(&self, time: DateTime<Utc>) {
            *self.time.lock().unwrap() = time;
        }
    }

    impl Clock for TestClock {
        fn now(&self) -> DateTime<Utc> {
            *self.time.lock().unwrap()
        }
    }

    #[tokio::test]
    async fn initial_call_to_consensus_params_fwd_to_api() {
        // given
        let mut api = MockCacheableRpcs::new();
        api.expect_consensus_parameters()
            .once()
            .return_once(|| Ok(ConsensusParameters::default()));
        let sut = CachedClient::new(api, TtlConfig::default(), TestClock::default());

        // when
        let _consensus_params = sut.consensus_parameters().await.unwrap();

        // then
        // mock validates the call went through
    }

    #[tokio::test]
    async fn new_call_to_consensus_params_cached() {
        // given
        let mut api = MockCacheableRpcs::new();
        api.expect_consensus_parameters()
            .once()
            .return_once(|| Ok(ConsensusParameters::default()));
        let sut = CachedClient::new(
            api,
            TtlConfig {
                consensus_parameters: Duration::from_secs(10),
            },
            TestClock::default(),
        );
        let consensus_parameters = sut.consensus_parameters().await.unwrap();

        // when
        let second_call_consensus_params = sut.consensus_parameters().await.unwrap();

        // then
        // mock validates only one call
        assert_eq!(consensus_parameters, second_call_consensus_params);
    }

    #[tokio::test]
    async fn if_ttl_expired_cache_is_updated() {
        // given
        let original_consensus_params = ConsensusParameters::default();

        let changed_consensus_params = {
            let mut params = original_consensus_params.clone();
            params.set_chain_id(ChainId::new(99));
            params
        };

        let api = {
            let mut api = MockCacheableRpcs::new();
            let original_consensus_params = original_consensus_params.clone();
            let changed_consensus_params = changed_consensus_params.clone();
            api.expect_consensus_parameters()
                .once()
                .return_once(move || Ok(original_consensus_params));

            api.expect_consensus_parameters()
                .once()
                .return_once(move || Ok(changed_consensus_params));
            api
        };

        let clock = TestClock::default();
        let start_time = clock.now();

        let sut = CachedClient::new(
            api,
            TtlConfig {
                consensus_parameters: Duration::from_secs(10),
            },
            clock.clone(),
        );
        let consensus_parameters = sut.consensus_parameters().await.unwrap();

        clock.update_time(start_time + Duration::from_secs(11));
        // when
        let second_call_consensus_params = sut.consensus_parameters().await.unwrap();

        // then
        // mock validates two calls made
        assert_eq!(consensus_parameters, original_consensus_params);
        assert_eq!(second_call_consensus_params, changed_consensus_params);
    }

    #[tokio::test]
    async fn clear_cache_clears_consensus_params_cache() {
        // given
        let first_params = ConsensusParameters::default();
        let second_params = {
            let mut params = ConsensusParameters::default();
            params.set_chain_id(ChainId::new(1234));
            params
        };

        let api = {
            let mut api = MockCacheableRpcs::new();
            let first_clone = first_params.clone();
            api.expect_consensus_parameters()
                .times(1)
                .return_once(move || Ok(first_clone));

            let second_clone = second_params.clone();
            api.expect_consensus_parameters()
                .times(1)
                .return_once(move || Ok(second_clone));
            api
        };

        let clock = TestClock::default();
        let sut = CachedClient::new(api, TtlConfig::default(), clock.clone());

        let result1 = sut.consensus_parameters().await.unwrap();

        // when
        sut.clear().await;

        // then
        let result2 = sut.consensus_parameters().await.unwrap();

        assert_eq!(result1, first_params);
        assert_eq!(result2, second_params);
    }

    fn dummy_node_info() -> NodeInfo {
        NodeInfo {
            utxo_validation: true,
            vm_backtrace: false,
            max_tx: u64::MAX,
            max_gas: u64::MAX,
            max_size: u64::MAX,
            max_depth: u64::MAX,
            node_version: "0.0.1".to_string(),
            indexation: IndexationFlags {
                balances: true,
                coins_to_spend: true,
                asset_metadata: true,
            },
            tx_pool_stats: TxPoolStats {
                tx_count: U64(1),
                total_gas: U64(1),
                total_size: U64(1),
            },
        }
    }

    #[tokio::test]
    async fn initial_call_to_node_info_fwd_to_api() {
        // given
        let mut api = MockCacheableRpcs::new();
        api.expect_node_info()
            .once()
            .return_once(|| Ok(dummy_node_info()));
        let sut = CachedClient::new(api, TtlConfig::default(), TestClock::default());

        // when
        let _node_info = sut.node_info().await.unwrap();

        // then
        // The mock verifies that the API call was made.
    }

    #[tokio::test]
    async fn new_call_to_node_info_cached() {
        // given
        let mut api = MockCacheableRpcs::new();
        api.expect_node_info()
            .once()
            .return_once(|| Ok(dummy_node_info()));
        let sut = CachedClient::new(
            api,
            TtlConfig {
                consensus_parameters: Duration::from_secs(10),
            },
            TestClock::default(),
        );
        let first_node_info = sut.node_info().await.unwrap();

        // when: second call should return the cached value
        let second_node_info = sut.node_info().await.unwrap();

        // then: only one API call should have been made and the values are equal
        assert_eq!(first_node_info, second_node_info);
    }

    #[tokio::test]
    async fn if_ttl_expired_node_info_cache_is_updated() {
        // given
        let original_node_info = dummy_node_info();

        let changed_node_info = NodeInfo {
            node_version: "changed".to_string(),
            ..dummy_node_info()
        };

        let api = {
            let mut api = MockCacheableRpcs::new();
            let original_clone = original_node_info.clone();
            api.expect_node_info()
                .times(1)
                .return_once(move || Ok(original_clone));

            let changed_clone = changed_node_info.clone();
            api.expect_node_info()
                .times(1)
                .return_once(move || Ok(changed_clone));
            api
        };

        let clock = TestClock::default();
        let start_time = clock.now();

        let sut = CachedClient::new(
            api,
            TtlConfig {
                consensus_parameters: Duration::from_secs(10),
            },
            clock.clone(),
        );
        let first_call = sut.node_info().await.unwrap();

        // Advance time past the TTL.
        clock.update_time(start_time + Duration::from_secs(11));

        // when: a new API call should be triggered because the TTL expired
        let second_call = sut.node_info().await.unwrap();

        // then
        assert_eq!(first_call, original_node_info);
        assert_eq!(second_call, changed_node_info);
    }
}
