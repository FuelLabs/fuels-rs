mod storage;

use std::fmt::Debug;

use fuel_tx::{Bytes32, Contract as FuelContract, ContractId, Salt, StorageSlot};
pub use storage::*;

#[derive(Debug, Clone, PartialEq)]
pub struct Contract<Code> {
    code: Code,
    salt: Salt,
    storage_slots: Vec<StorageSlot>,
}

impl<T> Contract<T> {
    pub fn salt(&self) -> Salt {
        self.salt
    }

    pub fn with_salt(mut self, salt: impl Into<Salt>) -> Self {
        self.salt = salt.into();
        self
    }

    pub fn storage_slots(&self) -> &[StorageSlot] {
        &self.storage_slots
    }

    pub fn with_storage_slots(mut self, storage_slots: Vec<StorageSlot>) -> Self {
        self.storage_slots = storage_slots;
        self
    }
}

mod regular;
pub use regular::*;

mod loader;
pub use loader::*;

fn compute_contract_id_and_state_root(
    binary: &[u8],
    salt: &Salt,
    storage_slots: &[StorageSlot],
) -> (ContractId, Bytes32, Bytes32) {
    let fuel_contract = FuelContract::from(binary);
    let code_root = fuel_contract.root();
    let state_root = FuelContract::initial_state_root(storage_slots.iter());

    let contract_id = fuel_contract.id(salt, &code_root, &state_root);

    (contract_id, code_root, state_root)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use fuels_core::types::errors::Error;
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn autoload_storage_slots() {
        // given
        let temp_dir = tempdir().unwrap();
        let contract_bin = temp_dir.path().join("my_contract.bin");
        std::fs::write(&contract_bin, "").unwrap();

        let storage_file = temp_dir.path().join("my_contract-storage_slots.json");

        let expected_storage_slots = vec![StorageSlot::new([1; 32].into(), [2; 32].into())];
        save_slots(&expected_storage_slots, &storage_file);

        let storage_config = StorageConfiguration::new(true, vec![]);
        let load_config = LoadConfiguration::default().with_storage_configuration(storage_config);

        // when
        let loaded_contract = Contract::load_from(&contract_bin, load_config).unwrap();

        // then
        assert_eq!(loaded_contract.storage_slots, expected_storage_slots);
    }

    #[test]
    fn autoload_fails_if_file_missing() {
        // given
        let temp_dir = tempdir().unwrap();
        let contract_bin = temp_dir.path().join("my_contract.bin");
        std::fs::write(&contract_bin, "").unwrap();

        let storage_config = StorageConfiguration::new(true, vec![]);
        let load_config = LoadConfiguration::default().with_storage_configuration(storage_config);

        // when
        let error = Contract::load_from(&contract_bin, load_config)
            .expect_err("should have failed because the storage slots file is missing");

        // then
        let storage_slots_path = temp_dir.path().join("my_contract-storage_slots.json");
        let Error::Other(msg) = error else {
            panic!("expected an error of type `Other`");
        };
        assert_eq!(msg, format!("could not autoload storage slots from file: {storage_slots_path:?}. Either provide the file or disable autoloading in `StorageConfiguration`"));
    }

    fn save_slots(slots: &Vec<StorageSlot>, path: &Path) {
        std::fs::write(
            path,
            serde_json::to_string::<Vec<StorageSlot>>(slots).unwrap(),
        )
        .unwrap()
    }
}
