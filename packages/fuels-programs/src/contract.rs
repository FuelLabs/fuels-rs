mod storage;

use std::{
    borrow::Cow,
    collections::HashSet,
    fmt::Debug,
    fs,
    path::{Path, PathBuf},
};

use fuel_asm::{op, Instruction, RegId};
use fuel_tx::{Bytes32, Contract as FuelContract, ContractId, Salt, StorageSlot};
use fuels_accounts::{provider::Provider, Account};
use fuels_core::{
    constants::WORD_SIZE,
    types::{
        bech32::Bech32ContractId,
        errors::{error, Result},
        transaction::TxPolicies,
        transaction_builders::{
            Blob, BlobId, BlobTransactionBuilder, CreateTransactionBuilder, TransactionBuilder,
        },
    },
};
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

// impl Contract {
//     /// The contract code has been uploaded in blobs with [`BlobId`]s specified in `blob_ids`. This will create a loader
//     /// contract that, when deployed and executed, will load all the specified blobs into memory and delegate the call to the code contained in the blobs.
//     pub fn new_loader(
//         blob_ids: &[BlobId],
//         salt: Salt,
//         storage_slots: Vec<StorageSlot>,
//     ) -> Result<Self> {
//         // Loader asm code relies on there being at least one blob
//         if blob_ids.is_empty() {
//             return Err(error!(Other, "must provide at least one blob"));
//         }
//
//         let code = Self::loader_contract(blob_ids)?;
//         Ok(Self::new(code, salt, storage_slots))
//     }
//
//     /// Splits the contract into blobs, submits them, and awaits confirmation. Then, it deploys a loader contract.
//     /// This loader contract will load the blobs into memory and delegate the call to the code contained within the blobs.
//     /// This method is useful for deploying large contracts.
//     pub async fn deploy_as_loader(
//         self,
//         account: &impl Account,
//         tx_policies: TxPolicies,
//         blob_size_policy: BlobSizePolicy,
//     ) -> Result<Bech32ContractId> {
//         let provider = account.try_provider()?;
//
//         let blobs = self.generate_blobs(provider, blob_size_policy).await?;
//         let all_blob_ids = blobs.iter().map(|blob| blob.id()).collect::<Vec<_>>();
//         let mut already_uploaded = HashSet::new();
//
//         for blob in blobs {
//             let id = blob.id();
//
//             if already_uploaded.contains(&id) {
//                 continue;
//             }
//
//             let mut tb = BlobTransactionBuilder::default()
//                 .with_blob(blob)
//                 .with_tx_policies(tx_policies)
//                 .with_max_fee_estimation_tolerance(0.05);
//
//             account.adjust_for_fee(&mut tb, 0).await?;
//             account.add_witnesses(&mut tb)?;
//
//             let tx = tb.build(provider).await?;
//             provider
//                 .send_transaction_and_await_commit(tx)
//                 .await?
//                 .check(None)?;
//
//             already_uploaded.insert(id);
//         }
//
//         Self::new_loader(&all_blob_ids, self.salt, self.storage_slots)?
//             .deploy(account, tx_policies)
//             .await
//     }
//
//     /// Splits the contract binary into blobs based on the size specified by `blob_size_policy`.
//     /// This is useful if you prefer to manually deploy the blobs. Once uploaded, you can use [`Contract::new_loader`] to create a loader contract.
//     pub async fn generate_blobs(
//         &self,
//         provider: &Provider,
//         policy: BlobSizePolicy,
//     ) -> Result<Vec<Blob>> {
//         let blob_size = policy.resolve_size(provider).await?;
//
//         let blobs = self
//             .binary
//             .chunks(blob_size)
//             .map(|chunk| Blob::new(chunk.to_vec()))
//             .collect();
//
//         Ok(blobs)
//     }
//
//
//     pub fn load_from(binary_filepath: impl AsRef<Path>, config: LoadConfiguration) -> Result<Self> {
//         let binary_filepath = binary_filepath.as_ref();
//         validate_path_and_extension(binary_filepath, "bin")?;
//
//         let mut binary = fs::read(binary_filepath).map_err(|e| {
//             std::io::Error::new(
//                 e.kind(),
//                 format!("failed to read binary: {binary_filepath:?}: {e}"),
//             )
//         })?;
//
//         config.configurables.update_constants_in(&mut binary);
//
//         let storage_slots = Self::determine_storage_slots(config.storage, binary_filepath)?;
//
//         Ok(Self::new(binary, config.salt, storage_slots))
//     }
// }

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
    use fuels_accounts::wallet::WalletUnlocked;
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
