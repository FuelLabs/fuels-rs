use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::time::{Duration, SystemTime};

use fuel_types::AssetId;
use fuels_core::types::bech32::Bech32Address;
use fuels_core::types::coin_type::CoinTypeId;

type CoinCacheKey = (Bech32Address, AssetId);

#[derive(Debug)]
pub struct CoinsCache {
    pub ttl: Duration,
    pub items: HashMap<CoinCacheKey, HashSet<CoinCacheItem>>,
}

impl CoinsCache {
    pub fn new(ttl: Duration) -> Self {
        Self {
            ttl,
            items: HashMap::default(),
        }
    }

    pub fn append(&mut self, key: &CoinCacheKey, item: CoinCacheItem) {
        let items = self.items
            .entry(key.clone()).or_default();
        items.insert(item);
    }

    pub fn get_active(&mut self, key: &CoinCacheKey) -> HashSet<CoinTypeId> {
        // remove expired entries
        self.items
            .get_mut(key)
            .map(|entry| entry.retain(|item| item.is_valid(self.ttl)));

        self.items
            .get(key)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .map(|item| item.id)
            .collect()
    }
}

#[derive(Eq, Debug, Clone)]
pub struct CoinCacheItem {
    created_at: SystemTime,
    pub id: CoinTypeId,
}

impl PartialEq for CoinCacheItem {
    fn eq(&self, other: &Self) -> bool {
        self.id.eq(&other.id)
    }
}

impl Hash for CoinCacheItem {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl CoinCacheItem {
    pub fn new(id: CoinTypeId) -> Self {
        Self {
            created_at: SystemTime::now(),
            id,
        }
    }

    pub fn is_valid(&self, ttl: Duration) -> bool {
        self.created_at + ttl > SystemTime::now()
    }
}

#[cfg(test)]
mod tests {
    use fuel_tx::UtxoId;
    use fuel_types::{Bytes32, Nonce};

    use super::*;

    fn get_items() -> (CoinCacheItem, CoinCacheItem) {
        let id1 = Bytes32::from([1u8; 32]);

        let utxo_id = UtxoId::new(Bytes32::from([1u8; 32]), 0);
        let nonce = Nonce::new([2u8; 32]);

        let item1 = CoinCacheItem::new(CoinTypeId::UtxoId(utxo_id));
        let item2 = CoinCacheItem::new(CoinTypeId::Nonce(nonce));

        (item1, item2)
    }

    #[test]
    fn test_insert_and_get_active() {
        let mut cache = CoinsCache::new(Duration::from_secs(60));

        let key = Default::default();
        let (item1, item2) = get_items();

        cache.append(&key, item1.clone());
        cache.append(&key, item2.clone());

        let active_coins = cache.get_active(&key);

        assert_eq!(active_coins.len(), 2);
        assert!(active_coins.contains(&item1.id));
        assert!(active_coins.contains(&item2.id));
    }

    #[test]
    fn test_insert_and_expire_items() {
        let mut cache = CoinsCache::new(Duration::from_secs(1));

        let key = Default::default();
        let (item1, _) = get_items();

        cache.append(&key, item1.clone());

        // Sleep for more than the cache's TTL
        std::thread::sleep(Duration::from_secs(2));

        let (_, item2) = get_items();
        cache.append(&key, item2.clone());

        let active_coins = cache.get_active(&key);

        assert_eq!(active_coins.len(), 1);
        assert!(!active_coins.contains(&item1.id));
        assert!(active_coins.contains(&item2.id));
    }

    #[test]
    fn test_get_active_no_items() {
        let mut cache = CoinsCache::new(Duration::from_secs(60));

        let key = Default::default();
        let active_coins = cache.get_active(&key);

        assert!(active_coins.is_empty());
    }
}
