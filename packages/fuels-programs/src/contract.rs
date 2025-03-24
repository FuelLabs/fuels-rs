mod storage;

use std::fmt::Debug;

use fuel_tx::{Bytes32, Contract as FuelContract, ContractId, Salt, StorageSlot};
pub use storage::*;

/// Represents a contract that can be deployed either directly ([`Contract::regular`]) or through a loader [`Contract::convert_to_loader`].
/// Provides the ability to calculate the `ContractId` ([`Contract::contract_id`]) without needing to deploy the contract.
/// This struct also manages contract code updates with `configurable`s
/// ([`Contract::with_configurables`]) and can automatically
/// load storage slots (via [`Contract::load_from`]).
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
// reexported to avoid doing a breaking change
pub use loader::*;

pub use crate::assembly::contract_call::loader_contract_asm;

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

    use fuels_core::types::{
        errors::{Error, Result},
        transaction_builders::Blob,
    };
    use tempfile::tempdir;

    use super::*;
    use crate::assembly::contract_call::loader_contract_asm;

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
        assert_eq!(
            msg,
            format!(
                "could not autoload storage slots from file: {storage_slots_path:?}. Either provide the file or disable autoloading in `StorageConfiguration`"
            )
        );
    }

    fn save_slots(slots: &Vec<StorageSlot>, path: &Path) {
        std::fs::write(
            path,
            serde_json::to_string::<Vec<StorageSlot>>(slots).unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn blob_size_must_be_greater_than_zero() {
        // given
        let contract = Contract::regular(vec![0x00], Salt::zeroed(), vec![]);

        // when
        let err = contract
            .convert_to_loader(0)
            .expect_err("should have failed because blob size is 0");

        // then
        assert_eq!(
            err.to_string(),
            "blob size must be greater than 0".to_string()
        );
    }

    #[test]
    fn contract_with_no_code_cannot_be_turned_into_a_loader() {
        // given
        let contract = Contract::regular(vec![], Salt::zeroed(), vec![]);

        // when
        let err = contract
            .convert_to_loader(100)
            .expect_err("should have failed because there is no code");

        // then
        assert_eq!(
            err.to_string(),
            "must provide at least one blob".to_string()
        );
    }

    #[test]
    fn loader_needs_at_least_one_blob() {
        // given
        let no_blobs = vec![];

        // when
        let err = Contract::loader_from_blobs(no_blobs, Salt::default(), vec![])
            .expect_err("should have failed because there are no blobs");

        // then
        assert_eq!(
            err.to_string(),
            "must provide at least one blob".to_string()
        );
    }

    #[test]
    fn loader_requires_all_except_the_last_blob_to_be_word_sized() {
        // given
        let blobs = [vec![0; 9], vec![0; 8]].map(Blob::new).to_vec();

        // when
        let err = Contract::loader_from_blobs(blobs, Salt::default(), vec![])
            .expect_err("should have failed because the first blob is not word-sized");

        // then
        assert_eq!(
            err.to_string(),
            "blob 1/2 has a size of 9 bytes, which is not a multiple of 8".to_string()
        );
    }

    #[test]
    fn last_blob_in_loader_can_be_unaligned() {
        // given
        let blobs = [vec![0; 8], vec![0; 9]].map(Blob::new).to_vec();

        // when
        let result = Contract::loader_from_blobs(blobs, Salt::default(), vec![]);

        // then
        let _ = result.unwrap();
    }

    #[test]
    fn can_load_regular_contract() -> Result<()> {
        // given
        let tmp_dir = tempfile::tempdir()?;
        let code_file = tmp_dir.path().join("contract.bin");
        let code = b"some fake contract code";
        std::fs::write(&code_file, code)?;

        // when
        let contract = Contract::load_from(
            code_file,
            LoadConfiguration::default()
                .with_storage_configuration(StorageConfiguration::default().with_autoload(false)),
        )?;

        // then
        assert_eq!(contract.code(), code);

        Ok(())
    }

    #[test]
    fn can_manually_create_regular_contract() -> Result<()> {
        // given
        let binary = b"some fake contract code";

        // when
        let contract = Contract::regular(binary.to_vec(), Salt::zeroed(), vec![]);

        // then
        assert_eq!(contract.code(), binary);

        Ok(())
    }

    macro_rules! getters_work {
        ($contract: ident, $contract_id: expr, $state_root: expr, $code_root: expr, $salt: expr, $code: expr) => {
            assert_eq!($contract.contract_id(), $contract_id);
            assert_eq!($contract.state_root(), $state_root);
            assert_eq!($contract.code_root(), $code_root);
            assert_eq!($contract.salt(), $salt);
            assert_eq!($contract.code(), $code);
        };
    }

    #[test]
    fn regular_contract_has_expected_getters() -> Result<()> {
        let contract_binary = b"some fake contract code";
        let storage_slots = vec![StorageSlot::new([2; 32].into(), [1; 32].into())];
        let contract = Contract::regular(contract_binary.to_vec(), Salt::zeroed(), storage_slots);

        let expected_contract_id =
            "93c9f1e61efb25458e3c56fdcfee62acb61c0533364eeec7ba61cb2957aa657b".parse()?;
        let expected_state_root =
            "852b7b7527124dbcd44302e52453b864dc6f4d9544851c729da666a430b84c97".parse()?;
        let expected_code_root =
            "69ca130191e9e469f1580229760b327a0729237f1aff65cf1d076b2dd8360031".parse()?;
        let expected_salt = Salt::zeroed();

        getters_work!(
            contract,
            expected_contract_id,
            expected_state_root,
            expected_code_root,
            expected_salt,
            contract_binary
        );

        Ok(())
    }

    #[test]
    fn regular_can_be_turned_into_loader_and_back() -> Result<()> {
        let contract_binary = b"some fake contract code";

        let contract_original = Contract::regular(contract_binary.to_vec(), Salt::zeroed(), vec![]);

        let loader_contract = contract_original.clone().convert_to_loader(1)?;

        let regular_recreated = loader_contract.clone().revert_to_regular();

        assert_eq!(regular_recreated, contract_original);

        Ok(())
    }

    #[test]
    fn unuploaded_loader_contract_has_expected_getters() -> Result<()> {
        let contract_binary = b"some fake contract code";

        let storage_slots = vec![StorageSlot::new([2; 32].into(), [1; 32].into())];
        let original = Contract::regular(contract_binary.to_vec(), Salt::zeroed(), storage_slots);
        let loader = original.clone().convert_to_loader(1024)?;

        let loader_asm = loader_contract_asm(&loader.blob_ids()).unwrap();
        let manual_loader = original.with_code(loader_asm);

        getters_work!(
            loader,
            manual_loader.contract_id(),
            manual_loader.state_root(),
            manual_loader.code_root(),
            manual_loader.salt(),
            manual_loader.code()
        );

        Ok(())
    }

    #[test]
    fn unuploaded_loader_requires_at_least_one_blob() -> Result<()> {
        // given
        let no_blob_ids = vec![];

        // when
        let loader = Contract::loader_from_blob_ids(no_blob_ids, Salt::default(), vec![])
            .expect_err("should have failed because there are no blobs");

        // then
        assert_eq!(
            loader.to_string(),
            "must provide at least one blob".to_string()
        );
        Ok(())
    }

    #[test]
    fn uploaded_loader_has_expected_getters() -> Result<()> {
        let contract_binary = b"some fake contract code";
        let original_contract = Contract::regular(contract_binary.to_vec(), Salt::zeroed(), vec![]);

        let blob_ids = original_contract
            .clone()
            .convert_to_loader(1024)?
            .blob_ids();

        // we pretend we uploaded the blobs
        let loader = Contract::loader_from_blob_ids(blob_ids.clone(), Salt::default(), vec![])?;

        let loader_asm = loader_contract_asm(&blob_ids).unwrap();
        let manual_loader = original_contract.with_code(loader_asm);

        getters_work!(
            loader,
            manual_loader.contract_id(),
            manual_loader.state_root(),
            manual_loader.code_root(),
            manual_loader.salt(),
            manual_loader.code()
        );

        Ok(())
    }
}
