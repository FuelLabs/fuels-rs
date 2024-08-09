use std::{
    collections::HashMap,
    default::Default,
    fmt::Debug,
    io,
    path::{Path, PathBuf},
};

use fuel_tx::{Bytes32, StorageSlot};
use fuels_core::types::errors::{error, Result};

/// Configuration for contract storage
#[derive(Debug, Clone)]
pub struct StorageConfiguration {
    autoload_storage: bool,
    slot_overrides: StorageSlots,
}

impl Default for StorageConfiguration {
    fn default() -> Self {
        Self {
            autoload_storage: true,
            slot_overrides: Default::default(),
        }
    }
}

impl StorageConfiguration {
    pub fn new(autoload_enabled: bool, slots: impl IntoIterator<Item = StorageSlot>) -> Self {
        let config = Self {
            autoload_storage: autoload_enabled,
            slot_overrides: Default::default(),
        };

        config.add_slot_overrides(slots)
    }

    /// If enabled will try to automatically discover and load the storage configuration from the
    /// storage config json file.
    pub fn with_autoload(mut self, enabled: bool) -> Self {
        self.autoload_storage = enabled;
        self
    }

    pub fn autoload_enabled(&self) -> bool {
        self.autoload_storage
    }

    /// Slots added via [`add_slot_overrides`] will override any
    /// existing slots with matching keys.
    pub fn add_slot_overrides(
        mut self,
        storage_slots: impl IntoIterator<Item = StorageSlot>,
    ) -> Self {
        self.slot_overrides.add_overrides(storage_slots);
        self
    }

    /// Slots added via [`add_slot_overrides_from_file`] will override any
    /// existing slots with matching keys.
    ///
    /// `path` - path to a JSON file containing the storage slots.
    pub fn add_slot_overrides_from_file(mut self, path: impl AsRef<Path>) -> Result<Self> {
        let slots = StorageSlots::load_from_file(path.as_ref())?;
        self.slot_overrides.add_overrides(slots.into_iter());
        Ok(self)
    }

    pub fn into_slots(self) -> impl Iterator<Item = StorageSlot> {
        self.slot_overrides.into_iter()
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct StorageSlots {
    storage_slots: HashMap<Bytes32, StorageSlot>,
}

impl StorageSlots {
    fn from(storage_slots: impl IntoIterator<Item = StorageSlot>) -> Self {
        let pairs = storage_slots.into_iter().map(|slot| (*slot.key(), slot));
        Self {
            storage_slots: pairs.collect(),
        }
    }

    pub(crate) fn add_overrides(
        &mut self,
        storage_slots: impl IntoIterator<Item = StorageSlot>,
    ) -> &mut Self {
        let pairs = storage_slots.into_iter().map(|slot| (*slot.key(), slot));
        self.storage_slots.extend(pairs);
        self
    }

    pub(crate) fn load_from_file(storage_path: impl AsRef<Path>) -> Result<Self> {
        let storage_path = storage_path.as_ref();
        validate_path_and_extension(storage_path, "json")?;

        let storage_json_string = std::fs::read_to_string(storage_path).map_err(|e| {
            io::Error::new(
                e.kind(),
                format!("failed to read storage slots from: {storage_path:?}: {e}"),
            )
        })?;

        let decoded_slots = serde_json::from_str::<Vec<StorageSlot>>(&storage_json_string)?;

        Ok(StorageSlots::from(decoded_slots))
    }

    pub(crate) fn into_iter(self) -> impl Iterator<Item = StorageSlot> {
        self.storage_slots.into_values()
    }
}

pub(crate) fn determine_storage_slots(
    storage_config: StorageConfiguration,
    binary_filepath: &Path,
) -> Result<Vec<StorageSlot>> {
    let autoload_enabled = storage_config.autoload_enabled();
    let user_overrides = storage_config.into_slots().collect::<Vec<_>>();
    let slots = if autoload_enabled {
        let mut slots = autoload_storage_slots(binary_filepath)?;
        slots.add_overrides(user_overrides);
        slots.into_iter().collect()
    } else {
        user_overrides
    };

    Ok(slots)
}

pub(crate) fn autoload_storage_slots(contract_binary: &Path) -> Result<StorageSlots> {
    let storage_file = expected_storage_slots_filepath(contract_binary)
        .ok_or_else(|| error!(Other, "could not determine storage slots file"))?;

    StorageSlots::load_from_file(&storage_file)
                .map_err(|_| error!(Other, "could not autoload storage slots from file: {storage_file:?}. \
                                    Either provide the file or disable autoloading in `StorageConfiguration`"))
}

pub(crate) fn expected_storage_slots_filepath(contract_binary: &Path) -> Option<PathBuf> {
    let dir = contract_binary.parent()?;

    let binary_filename = contract_binary.file_stem()?.to_str()?;

    Some(dir.join(format!("{binary_filename}-storage_slots.json")))
}
pub(crate) fn validate_path_and_extension(file_path: &Path, extension: &str) -> Result<()> {
    if !file_path.exists() {
        return Err(error!(IO, "file {file_path:?} does not exist"));
    }

    let path_extension = file_path
        .extension()
        .ok_or_else(|| error!(Other, "could not extract extension from: {file_path:?}"))?;

    if extension != path_extension {
        return Err(error!(
            Other,
            "expected {file_path:?} to have '.{extension}' extension"
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn merging_overrides_storage_slots() {
        // given
        let make_slot = |id, value| StorageSlot::new([id; 32].into(), [value; 32].into());

        let slots = (1..3).map(|id| make_slot(id, 100));
        let original_config = StorageConfiguration::new(false, slots);

        let overlapping_slots = (2..4).map(|id| make_slot(id, 200));

        // when
        let original_config = original_config.add_slot_overrides(overlapping_slots);

        // then
        assert_eq!(
            HashSet::from_iter(original_config.slot_overrides.into_iter()),
            HashSet::from([make_slot(1, 100), make_slot(2, 200), make_slot(3, 200)])
        );
    }
}
