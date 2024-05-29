use std::{
    collections::HashMap,
    default::Default,
    fmt::Debug,
    fs, io,
    path::{Path, PathBuf},
};

use fuel_tx::{AssetId, Bytes32, Contract as FuelContract, ContractId, Salt, StorageSlot};
use fuels_accounts::Account;
use fuels_core::{
    codec::LogDecoder,
    constants::DEFAULT_CALL_PARAMS_AMOUNT,
    types::{
        bech32::Bech32ContractId,
        errors::{error, Result},
        transaction::TxPolicies,
        transaction_builders::CreateTransactionBuilder,
    },
    Configurables,
};

#[derive(Debug, Clone)]
pub struct CallParameters {
    amount: u64,
    asset_id: Option<AssetId>,
    gas_forwarded: Option<u64>,
}

impl CallParameters {
    pub fn new(amount: u64, asset_id: AssetId, gas_forwarded: u64) -> Self {
        Self {
            amount,
            asset_id: Some(asset_id),
            gas_forwarded: Some(gas_forwarded),
        }
    }

    pub fn with_amount(mut self, amount: u64) -> Self {
        self.amount = amount;
        self
    }

    pub fn amount(&self) -> u64 {
        self.amount
    }

    pub fn with_asset_id(mut self, asset_id: AssetId) -> Self {
        self.asset_id = Some(asset_id);
        self
    }

    pub fn asset_id(&self) -> Option<AssetId> {
        self.asset_id
    }

    pub fn with_gas_forwarded(mut self, gas_forwarded: u64) -> Self {
        self.gas_forwarded = Some(gas_forwarded);
        self
    }

    pub fn gas_forwarded(&self) -> Option<u64> {
        self.gas_forwarded
    }
}

impl Default for CallParameters {
    fn default() -> Self {
        Self {
            amount: DEFAULT_CALL_PARAMS_AMOUNT,
            asset_id: None,
            gas_forwarded: None,
        }
    }
}

// Trait implemented by contract instances so that
// they can be passed to the `with_contracts` method
pub trait SettableContract {
    fn id(&self) -> Bech32ContractId;
    fn log_decoder(&self) -> LogDecoder;
}

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
struct StorageSlots {
    storage_slots: HashMap<Bytes32, StorageSlot>,
}

impl StorageSlots {
    fn from(storage_slots: impl IntoIterator<Item = StorageSlot>) -> Self {
        let pairs = storage_slots.into_iter().map(|slot| (*slot.key(), slot));
        Self {
            storage_slots: pairs.collect(),
        }
    }

    fn add_overrides(&mut self, storage_slots: impl IntoIterator<Item = StorageSlot>) -> &mut Self {
        let pairs = storage_slots.into_iter().map(|slot| (*slot.key(), slot));
        self.storage_slots.extend(pairs);
        self
    }

    fn load_from_file(storage_path: impl AsRef<Path>) -> Result<Self> {
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

    fn into_iter(self) -> impl Iterator<Item = StorageSlot> {
        self.storage_slots.into_values()
    }
}

/// Configuration for contract deployment
#[derive(Debug, Clone, Default)]
pub struct LoadConfiguration {
    storage: StorageConfiguration,
    configurables: Configurables,
    salt: Salt,
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
        );

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

fn validate_path_and_extension(file_path: &Path, extension: &str) -> Result<()> {
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

    use fuels_core::types::errors::Error;
    use tempfile::tempdir;

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
