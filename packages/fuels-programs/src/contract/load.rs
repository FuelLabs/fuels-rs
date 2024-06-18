use std::{default::Default, fmt::Debug};

use fuel_tx::Salt;
use fuels_core::Configurables;

use crate::contract::StorageConfiguration;

/// Configuration for contract deployment
#[derive(Debug, Clone, Default)]
pub struct LoadConfiguration {
    pub(crate) storage: StorageConfiguration,
    pub(crate) configurables: Configurables,
    pub(crate) salt: Salt,
}

impl LoadConfiguration {
    pub fn new(
        storage: StorageConfiguration,
        configurables: impl Into<Configurables>,
        salt: impl Into<Salt>,
    ) -> Self {
        Self {
            storage,
            configurables: configurables.into(),
            salt: salt.into(),
        }
    }

    pub fn with_storage_configuration(mut self, storage: StorageConfiguration) -> Self {
        self.storage = storage;
        self
    }

    pub fn with_configurables(mut self, configurables: impl Into<Configurables>) -> Self {
        self.configurables = configurables.into();
        self
    }

    pub fn with_salt(mut self, salt: impl Into<Salt>) -> Self {
        self.salt = salt.into();
        self
    }
}
