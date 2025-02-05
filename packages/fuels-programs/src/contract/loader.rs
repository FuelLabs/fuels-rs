use std::collections::HashSet;

use fuel_tx::{Bytes32, ContractId, Salt, StorageSlot};
use fuels_accounts::Account;
use fuels_core::{
    constants::WORD_SIZE,
    types::{
        errors::{error, Result},
        transaction::TxPolicies,
        transaction_builders::{Blob, BlobId, BlobTransactionBuilder, TransactionBuilder},
    },
};

use crate::{assembly::contract_call::loader_contract_asm, DEFAULT_MAX_FEE_ESTIMATION_TOLERANCE};

use super::{compute_contract_id_and_state_root, Contract, DeployResponse, Regular};

#[derive(Debug, Clone)]
pub struct BlobsUploaded {
    blob_ids: Vec<BlobId>,
}

#[derive(Debug, Clone)]
pub struct BlobsNotUploaded {
    blobs: Vec<Blob>,
}

#[derive(Debug, Clone)]
pub struct Loader<Blobs> {
    as_blobs: Blobs,
}

impl Contract<Loader<BlobsNotUploaded>> {
    pub fn code(&self) -> Vec<u8> {
        let ids: Vec<_> = self.blob_ids();
        loader_contract_asm(&ids)
            .expect("a contract to be creatable due to the check done in loader_from_blobs")
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

    /// Creates a loader contract for the code found in `blobs`. Calling `deploy` on this contract
    /// does two things:
    /// 1. Uploads the code blobs.
    /// 2. Deploys the loader contract.
    ///
    /// The loader contract, when executed, will load all the given blobs into memory and delegate the call to the original contract code contained in the blobs.
    pub fn loader_from_blobs(
        blobs: Vec<Blob>,
        salt: Salt,
        storage_slots: Vec<StorageSlot>,
    ) -> Result<Self> {
        if blobs.is_empty() {
            return Err(error!(Other, "must provide at least one blob"));
        }

        let idx_of_last_blob = blobs.len().saturating_sub(1);
        let idx_of_offender = blobs.iter().enumerate().find_map(|(idx, blob)| {
            (blob.len() % WORD_SIZE != 0 && idx != idx_of_last_blob).then_some(idx)
        });

        if let Some(idx) = idx_of_offender {
            return Err(error!(
                Other,
                "blob {}/{} has a size of {} bytes, which is not a multiple of {WORD_SIZE}",
                idx.saturating_add(1),
                blobs.len(),
                blobs[idx].len()
            ));
        }

        let ids = blobs.iter().map(|blob| blob.id()).collect::<Vec<_>>();

        // Validate that the loader contract can be created.
        loader_contract_asm(&ids)?;

        Ok(Self {
            code: Loader {
                as_blobs: BlobsNotUploaded { blobs },
            },
            salt,
            storage_slots,
        })
    }

    pub fn blobs(&self) -> &[Blob] {
        self.code.as_blobs.blobs.as_slice()
    }

    pub fn blob_ids(&self) -> Vec<BlobId> {
        self.code
            .as_blobs
            .blobs
            .iter()
            .map(|blob| blob.id())
            .collect()
    }

    /// Uploads the blobs associated with this contract. Calling `deploy` on the result will only
    /// deploy the loader contract.
    pub async fn upload_blobs(
        self,
        account: &impl Account,
        tx_policies: TxPolicies,
    ) -> Result<Contract<Loader<BlobsUploaded>>> {
        let provider = account.try_provider()?;

        let all_blob_ids = self.blob_ids();
        let mut already_uploaded = HashSet::new();

        for blob in self.code.as_blobs.blobs {
            let id = blob.id();

            if already_uploaded.contains(&id) {
                continue;
            }

            if provider.blob_exists(id).await? {
                already_uploaded.insert(id);
                continue;
            }

            let mut tb = BlobTransactionBuilder::default()
                .with_blob(blob)
                .with_tx_policies(tx_policies)
                .with_max_fee_estimation_tolerance(DEFAULT_MAX_FEE_ESTIMATION_TOLERANCE);

            account.adjust_for_fee(&mut tb, 0).await?;
            account.add_witnesses(&mut tb)?;

            let tx = tb.build(provider).await?;

            let tx_status_response = provider.send_transaction_and_await_commit(tx).await;
            tx_status_response.and_then(|response| response.check(None))?;

            already_uploaded.insert(id);
        }

        Contract::loader_from_blob_ids(all_blob_ids, self.salt, self.storage_slots)
    }

    /// Deploys the loader contract after uploading the code blobs.
    pub async fn deploy(
        self,
        account: &impl Account,
        tx_policies: TxPolicies,
    ) -> Result<DeployResponse> {
        self.upload_blobs(account, tx_policies)
            .await?
            .deploy(account, tx_policies)
            .await
    }

    /// Deploys the loader contract after uploading the code blobs,
    /// if there is no contract with this ContractId Already.
    pub async fn deploy_if_not_exists(
        self,
        account: &impl Account,
        tx_policies: TxPolicies,
    ) -> Result<DeployResponse> {
        self.upload_blobs(account, tx_policies)
            .await?
            .deploy_if_not_exists(account, tx_policies)
            .await
    }
    /// Reverts the contract from a loader contract back to a regular contract.
    pub fn revert_to_regular(self) -> Contract<Regular> {
        let code = self
            .code
            .as_blobs
            .blobs
            .into_iter()
            .flat_map(Vec::from)
            .collect();

        Contract::regular(code, self.salt, self.storage_slots)
    }
}

impl Contract<Loader<BlobsUploaded>> {
    pub fn code(&self) -> Vec<u8> {
        loader_contract_asm(&self.code.as_blobs.blob_ids)
            .expect("a contract to be creatable due to the check done in loader_for_blobs")
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

    pub fn compute_roots(&self) -> (ContractId, Bytes32, Bytes32) {
        compute_contract_id_and_state_root(&self.code(), &self.salt, &self.storage_slots)
    }

    /// Creates a loader contract using previously uploaded blobs.
    ///
    /// The contract code has been uploaded in blobs with [`BlobId`]s specified in `blob_ids`.
    /// This will create a loader contract that, when deployed and executed, will load all the specified blobs into memory and delegate the call to the code contained in the blobs.
    pub fn loader_from_blob_ids(
        blob_ids: Vec<BlobId>,
        salt: Salt,
        storage_slots: Vec<StorageSlot>,
    ) -> Result<Self> {
        if blob_ids.is_empty() {
            return Err(error!(Other, "must provide at least one blob"));
        }

        // Validate that the loader contract can be created.
        loader_contract_asm(&blob_ids)?;

        Ok(Self {
            code: Loader {
                as_blobs: BlobsUploaded { blob_ids },
            },
            salt,
            storage_slots,
        })
    }

    pub fn blob_ids(&self) -> &[BlobId] {
        &self.code.as_blobs.blob_ids
    }

    /// Deploys the loader contract.
    pub async fn deploy(
        self,
        account: &impl Account,
        tx_policies: TxPolicies,
    ) -> Result<DeployResponse> {
        Contract::regular(self.code(), self.salt, self.storage_slots)
            .deploy(account, tx_policies)
            .await
    }

    pub async fn deploy_if_not_exists(
        self,
        account: &impl Account,
        tx_policies: TxPolicies,
    ) -> Result<DeployResponse> {
        Contract::regular(self.code(), self.salt, self.storage_slots)
            .deploy_if_not_exists(account, tx_policies)
            .await
    }
}
