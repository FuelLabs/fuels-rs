use std::{
    collections::{HashMap, HashSet},
    hash::{Hash, Hasher},
};

use fuel_types::AssetId;
use fuels_core::types::{Address, coin_type_id::CoinTypeId};
use tokio::time::{Duration, Instant};

type CoinCacheKey = (Address, AssetId);

#[derive(Debug)]
pub(crate) struct CoinsCache {
    ttl: Duration,
    items: HashMap<CoinCacheKey, HashSet<CoinCacheItem>>,
}

impl Default for CoinsCache {
    fn default() -> Self {
        Self::new(Duration::from_secs(30))
    }
}

impl CoinsCache {
    pub fn new(ttl: Duration) -> Self {
        Self {
            ttl,
            items: HashMap::default(),
        }
    }

    pub fn insert_multiple(
        &mut self,
        coin_ids: impl IntoIterator<Item = (CoinCacheKey, Vec<CoinTypeId>)>,
    ) {
        for (key, ids) in coin_ids {
            let new_items = ids.into_iter().map(CoinCacheItem::new);

            let items = self.items.entry(key).or_default();
            items.extend(new_items);
        }
    }

    pub fn get_active(&mut self, key: &CoinCacheKey) -> HashSet<CoinTypeId> {
        self.remove_expired_entries(key);

        self.items
            .get(key)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .map(|item| item.id)
            .collect()
    }

    pub fn remove_items(
        &mut self,
        inputs: impl IntoIterator<Item = (CoinCacheKey, Vec<CoinTypeId>)>,
    ) {
        for (key, ids) in inputs {
            for id in ids {
                self.remove(&key, id);
            }
        }
    }

    fn remove(&mut self, key: &CoinCacheKey, id: CoinTypeId) {
        if let Some(ids) = self.items.get_mut(key) {
            let item = CoinCacheItem::new(id);
            ids.remove(&item);
        }
    }

    fn remove_expired_entries(&mut self, key: &CoinCacheKey) {
        if let Some(entry) = self.items.get_mut(key) {
            entry.retain(|item| item.is_valid(self.ttl));
        }
    }
}

#[derive(Eq, Debug, Clone)]
struct CoinCacheItem {
    created_at: Instant,
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
            created_at: Instant::now(),
            id,
        }
    }

    pub fn is_valid(&self, ttl: Duration) -> bool {
        self.created_at + ttl > Instant::now()
    }
}

#[cfg(test)]
mod tests {
    use fuel_tx::UtxoId;
    use fuel_types::{Bytes32, Nonce};

    use super::*;

    fn get_items() -> (CoinTypeId, CoinTypeId) {
        let utxo_id = UtxoId::new(Bytes32::from([1u8; 32]), 0);
        let nonce = Nonce::new([2u8; 32]);

        (CoinTypeId::UtxoId(utxo_id), CoinTypeId::Nonce(nonce))
    }

    #[test]
    fn test_insert_and_get_active() {
        let mut cache = CoinsCache::new(Duration::from_secs(60));

        let key: CoinCacheKey = Default::default();
        let (item1, item2) = get_items();
        let items = HashMap::from([(key, vec![item1.clone(), item2.clone()])]);

        cache.insert_multiple(items);

        let active_coins = cache.get_active(&key);

        assert_eq!(active_coins.len(), 2);
        assert!(active_coins.contains(&item1));
        assert!(active_coins.contains(&item2));
    }

    #[tokio::test]
    async fn test_insert_and_expire_items() {
        let mut cache = CoinsCache::new(Duration::from_secs(10));

        let key = CoinCacheKey::default();
        let (item1, _) = get_items();
        let items = HashMap::from([(key, vec![item1.clone()])]);

        cache.insert_multiple(items);

        // Advance time by more than the cache's TTL
        tokio::time::pause();
        tokio::time::advance(Duration::from_secs(12)).await;

        let (_, item2) = get_items();
        let items = HashMap::from([(key, vec![item2.clone()])]);
        cache.insert_multiple(items);

        let active_coins = cache.get_active(&key);

        assert_eq!(active_coins.len(), 1);
        assert!(!active_coins.contains(&item1));
        assert!(active_coins.contains(&item2));
    }

    #[test]
    fn test_get_active_no_items() {
        let mut cache = CoinsCache::new(Duration::from_secs(60));

        let key = Default::default();
        let active_coins = cache.get_active(&key);

        assert!(active_coins.is_empty());
    }

    #[test]
    fn test_remove_items() {
        let mut cache = CoinsCache::new(Duration::from_secs(60));

        let key: CoinCacheKey = Default::default();
        let (item1, item2) = get_items();

        let items_to_insert = [(key, vec![item1.clone(), item2.clone()])];
        cache.insert_multiple(items_to_insert.iter().cloned());

        let items_to_remove = [(key, vec![item1.clone()])];
        cache.remove_items(items_to_remove.iter().cloned());

        let active_coins = cache.get_active(&key);

        assert_eq!(active_coins.len(), 1);
        assert!(!active_coins.contains(&item1));
        assert!(active_coins.contains(&item2));
    }
}
