use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

use fuels_types::resource::{Resource, ResourceId};
use fuels_types::transaction::CachedTx;

#[derive(Clone, Debug, Default)]
pub struct ResourceCache {
    pub resource_ids_used: RefCell<HashSet<ResourceId>>,
    pub expected_resources: RefCell<HashMap<ResourceId, Resource>>,
}

impl ResourceCache {
    pub fn save(&self, cached_tx: CachedTx) {
        let mut resource_ids_used = self.resource_ids_used.borrow_mut();
        let mut expected_resources = self.expected_resources.borrow_mut();

        // Remove used resource ids from 'expected' as they have been retrieved successfully.
        cached_tx
            .resource_ids_used
            .into_iter()
            .for_each(|resource_id| {
                expected_resources.remove(&resource_id);
                resource_ids_used.insert(resource_id);
            });

        cached_tx
            .expected_resources
            .into_iter()
            .for_each(|resource| {
                expected_resources.insert(resource.id(), resource);
            });
    }

    pub fn get_used_resource_ids(&self) -> Vec<ResourceId> {
        self.resource_ids_used.borrow_mut().drain().collect()
    }

    pub fn get_expected_resources(&self) -> Vec<Resource> {
        self.expected_resources
            .borrow_mut()
            .drain()
            .map(|(_, resource)| resource)
            .collect()
    }
}
