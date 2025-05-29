use std::{default::Default, fmt::Debug, path::Path};

use fuel_tx::{StorageSlot, TxId};
use fuels_accounts::Account;
use fuels_core::{
    Configurables,
    constants::WORD_SIZE,
    error,
    types::{
        Bytes32, ContractId, Salt,
        errors::{Context, Result},
        transaction::{Transaction, TxPolicies},
        transaction_builders::{Blob, CreateTransactionBuilder},
        tx_status::Success,
    },
};

use super::{
    BlobsNotUploaded, Contract, Loader, StorageConfiguration, compute_contract_id_and_state_root,
    validate_path_and_extension,
};
use crate::DEFAULT_MAX_FEE_ESTIMATION_TOLERANCE;

#[derive(Clone, Debug)]
pub struct DeployResponse {
    pub tx_status: Option<Success>,
    pub tx_id: Option<TxId>,
    pub contract_id: ContractId,
}

// In a mod so that we eliminate the footgun of getting the private `code` field without applying
// configurables
mod code_types {
    use fuels_core::Configurables;

    #[derive(Debug, Clone, PartialEq)]
    pub struct Regular {
        code: Vec<u8>,
        configurables: Configurables,
    }

    impl Regular {
        pub(crate) fn new(code: Vec<u8>, configurables: Configurables) -> Self {
            Self {
                code,
                configurables,
            }
        }

        pub(crate) fn with_code(self, code: Vec<u8>) -> Self {
            Self { code, ..self }
        }

        pub(crate) fn with_configurables(self, configurables: Configurables) -> Self {
            Self {
                configurables,
                ..self
            }
        }

        pub(crate) fn code(&self) -> Vec<u8> {
            let mut code = self.code.clone();
            self.configurables.update_constants_in(&mut code);
            code
        }
    }
}
pub use code_types::*;

impl Contract<Regular> {
    pub fn with_code(self, code: Vec<u8>) -> Self {
        Self {
            code: self.code.with_code(code),
            salt: self.salt,
            storage_slots: self.storage_slots,
        }
    }

    pub fn with_configurables(self, configurables: impl Into<Configurables>) -> Self {
        Self {
            code: self.code.with_configurables(configurables.into()),
            ..self
        }
    }

    pub fn code(&self) -> Vec<u8> {
        self.code.code()
    }

    pub fn contract_id(&self) -> ContractId {
        self.compute_roots().0
    }

    pub fn code_root(&self) -> Bytes32 {
        self.compute_roots().1
    }

    pub fn state_root(&self) -> Bytes32 {
        self.compute_roots().2
    }

    fn compute_roots(&self) -> (ContractId, Bytes32, Bytes32) {
        compute_contract_id_and_state_root(&self.code(), &self.salt, &self.storage_slots)
    }

    /// Loads a contract from a binary file. Salt and storage slots are loaded as well, depending on the configuration provided.
    pub fn load_from(
        binary_filepath: impl AsRef<Path>,
        config: LoadConfiguration,
    ) -> Result<Contract<Regular>> {
        let binary_filepath = binary_filepath.as_ref();
        validate_path_and_extension(binary_filepath, "bin")?;

        let binary = std::fs::read(binary_filepath).map_err(|e| {
            std::io::Error::new(
                e.kind(),
                format!("failed to read binary: {binary_filepath:?}: {e}"),
            )
        })?;

        let storage_slots = super::determine_storage_slots(config.storage, binary_filepath)?;

        Ok(Contract {
            code: Regular::new(binary, config.configurables),
            salt: config.salt,
            storage_slots,
        })
    }

    /// Creates a regular contract with the given code, salt, and storage slots.
    pub fn regular(
        code: Vec<u8>,
        salt: Salt,
        storage_slots: Vec<StorageSlot>,
    ) -> Contract<Regular> {
        Contract {
            code: Regular::new(code, Configurables::default()),
            salt,
            storage_slots,
        }
    }

    /// Deploys a compiled contract to a running node.
    /// To deploy a contract, you need an account with enough assets to pay for deployment.
    /// This account will also receive the change.
    pub async fn deploy(
        self,
        account: &impl Account,
        tx_policies: TxPolicies,
    ) -> Result<DeployResponse> {
        let contract_id = self.contract_id();
        let state_root = self.state_root();
        let salt = self.salt;
        let storage_slots = self.storage_slots;

        let mut tb = CreateTransactionBuilder::prepare_contract_deployment(
            self.code.code(),
            contract_id,
            state_root,
            salt,
            storage_slots.to_vec(),
            tx_policies,
        )
        .with_max_fee_estimation_tolerance(DEFAULT_MAX_FEE_ESTIMATION_TOLERANCE);

        account.add_witnesses(&mut tb)?;
        account
            .adjust_for_fee(&mut tb, 0)
            .await
            .context("failed to adjust inputs to cover for missing base asset")?;

        let provider = account.try_provider()?;
        let consensus_parameters = provider.consensus_parameters().await?;

        let tx = tb.build(provider).await?;
        let tx_id = Some(tx.id(consensus_parameters.chain_id()));

        let tx_status = provider.send_transaction_and_await_commit(tx).await?;

        Ok(DeployResponse {
            tx_status: Some(tx_status.take_success_checked(None)?),
            tx_id,
            contract_id,
        })
    }

    /// Deploys a compiled contract to a running node if a contract with
    /// the corresponding [`ContractId`] doesn't exist.
    pub async fn deploy_if_not_exists(
        self,
        account: &impl Account,
        tx_policies: TxPolicies,
    ) -> Result<DeployResponse> {
        let contract_id = self.contract_id();
        let provider = account.try_provider()?;
        if provider.contract_exists(&contract_id).await? {
            Ok(DeployResponse {
                tx_status: None,
                tx_id: None,
                contract_id,
            })
        } else {
            self.deploy(account, tx_policies).await
        }
    }

    /// Converts a regular contract into a loader contract, splitting the code into blobs.
    pub fn convert_to_loader(
        self,
        max_words_per_blob: usize,
    ) -> Result<Contract<Loader<BlobsNotUploaded>>> {
        if max_words_per_blob == 0 {
            return Err(error!(Other, "blob size must be greater than 0"));
        }
        let blobs = self
            .code()
            .chunks(max_words_per_blob.saturating_mul(WORD_SIZE))
            .map(|chunk| Blob::new(chunk.to_vec()))
            .collect();

        Contract::loader_from_blobs(blobs, self.salt, self.storage_slots)
    }

    /// Deploys the contract either as a regular contract or as a loader contract if it exceeds the size limit.
    pub async fn smart_deploy(
        self,
        account: &impl Account,
        tx_policies: TxPolicies,
        max_words_per_blob: usize,
    ) -> Result<DeployResponse> {
        let provider = account.try_provider()?;
        let max_contract_size = provider
            .consensus_parameters()
            .await?
            .contract_params()
            .contract_max_size() as usize;

        if self.code().len() <= max_contract_size {
            self.deploy(account, tx_policies).await
        } else {
            self.convert_to_loader(max_words_per_blob)?
                .deploy(account, tx_policies)
                .await
        }
    }
}

/// Configuration for contract deployment.
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
