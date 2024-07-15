mod load;
mod storage;

use std::{
    fmt::Debug,
    fs,
    path::{Path, PathBuf},
};

use fuel_tx::{Bytes32, Contract as FuelContract, ContractId, Salt, StorageSlot};
use fuels_accounts::Account;
use fuels_core::types::{
    bech32::Bech32ContractId,
    errors::{error, Result},
    transaction::TxPolicies,
    transaction_builders::CreateTransactionBuilder,
};
pub use load::*;
pub use storage::*;

/// [`Contract`] is a struct to interface with a contract. That includes things such as
/// compiling, deploying, and running transactions against a contract.
#[derive(Debug, Clone)]
pub struct Contract {
    binary: Vec<u8>,
    salt: Salt,
    storage_slots: Vec<StorageSlot>,
    contract_id: ContractId,
    code_root: Bytes32,
    state_root: Bytes32,
}

impl Contract {
    pub fn new(binary: Vec<u8>, salt: Salt, storage_slots: Vec<StorageSlot>) -> Self {
        let (contract_id, code_root, state_root) =
            Self::compute_contract_id_and_state_root(&binary, &salt, &storage_slots);

        Self {
            binary,
            salt,
            storage_slots,
            contract_id,
            code_root,
            state_root,
        }
    }

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

    pub fn with_salt(self, salt: impl Into<Salt>) -> Self {
        Self::new(self.binary, salt.into(), self.storage_slots)
    }

    pub fn contract_id(&self) -> ContractId {
        self.contract_id
    }

    pub fn state_root(&self) -> Bytes32 {
        self.state_root
    }

    pub fn code_root(&self) -> Bytes32 {
        self.code_root
    }

    /// Deploys a compiled contract to a running node
    /// To deploy a contract, you need an account with enough assets to pay for deployment.
    /// This account will also receive the change.
    pub async fn deploy(
        self,
        account: &impl Account,
        tx_policies: TxPolicies,
    ) -> Result<Bech32ContractId> {
        let mut tb = CreateTransactionBuilder::prepare_contract_deployment(
            self.binary,
            self.contract_id,
            self.state_root,
            self.salt,
            self.storage_slots,
            tx_policies,
        )
        .with_max_fee_estimation_tolerance(0.05);

        account.add_witnesses(&mut tb)?;
        account.adjust_for_fee(&mut tb, 0).await?;

        let provider = account.try_provider()?;

        let tx = tb.build(provider).await?;

        provider
            .send_transaction_and_await_commit(tx)
            .await?
            .check(None)?;

        Ok(self.contract_id.into())
    }

    pub fn load_from(binary_filepath: impl AsRef<Path>, config: LoadConfiguration) -> Result<Self> {
        let binary_filepath = binary_filepath.as_ref();
        validate_path_and_extension(binary_filepath, "bin")?;

        let mut binary = fs::read(binary_filepath).map_err(|e| {
            std::io::Error::new(
                e.kind(),
                format!("failed to read binary: {binary_filepath:?}: {e}"),
            )
        })?;

        config.configurables.update_constants_in(&mut binary);

        let storage_slots = Self::determine_storage_slots(config.storage, binary_filepath)?;

        Ok(Self::new(binary, config.salt, storage_slots))
    }

    fn determine_storage_slots(
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
}

fn autoload_storage_slots(contract_binary: &Path) -> Result<StorageSlots> {
    let storage_file = expected_storage_slots_filepath(contract_binary)
        .ok_or_else(|| error!(Other, "could not determine storage slots file"))?;

    StorageSlots::load_from_file(&storage_file)
                .map_err(|_| error!(Other, "could not autoload storage slots from file: {storage_file:?}. \
                                    Either provide the file or disable autoloading in `StorageConfiguration`"))
}

fn expected_storage_slots_filepath(contract_binary: &Path) -> Option<PathBuf> {
    let dir = contract_binary.parent()?;

    let binary_filename = contract_binary.file_stem()?.to_str()?;

    Some(dir.join(format!("{binary_filename}-storage_slots.json")))
}

#[cfg(test)]
mod tests {
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
