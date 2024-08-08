use std::borrow::Cow;
use std::collections::HashSet;

use fuel_asm::{op, Instruction, RegId};
use fuel_tx::{Bytes32, Contract as FuelContract, ContractId, Salt, StorageSlot};
use fuels_accounts::{provider::Provider, Account};
use fuels_core::types::errors::Result;
use fuels_core::types::transaction_builders::{Blob, BlobId};
use fuels_core::{
    constants::WORD_SIZE,
    types::{
        bech32::Bech32ContractId,
        errors::error,
        transaction::TxPolicies,
        transaction_builders::{
            BlobTransactionBuilder, CreateTransactionBuilder, TransactionBuilder,
        },
    },
};

use super::{compute_contract_id_and_state_root, Contract, Regular};

// create a contract that loads the specified blobs into memory and delegates the call to the code contained in the blobs.
pub fn loader_contract_asm(blob_ids: &[BlobId]) -> Result<Vec<u8>> {
    const BLOB_ID_SIZE: u16 = 32;
    let get_instructions = |num_of_instructions, num_of_blobs| {
        // There are 2 main steps:
        // 1. Load the blob contents into memory
        // 2. Jump to the beginning of the memory where the blobs were loaded
        // After that the execution continues normally with the loaded contract reading our
        // prepared fn selector and jumps to the selected contract method.
        [
            // 1. load the blob contents into memory
            // find the start of the hardcoded blob ids, which are located after the code ends,
            op::move_(0x10, RegId::IS),
            // 0x10 to hold the address of the current blob id
            op::addi(0x10, 0x10, num_of_instructions * Instruction::SIZE as u16),
            // The contract is going to be loaded from the current value of SP onwards, save
            // the location into 0x16 so we can jump into it later on
            op::move_(0x16, RegId::SP),
            // loop counter
            op::movi(0x13, num_of_blobs),
            // LOOP starts here
            // 0x11 to hold the size of the current blob
            op::bsiz(0x11, 0x10),
            // push the blob contents onto the stack
            op::ldc(0x10, 0, 0x11, 1),
            // move on to the next blob
            op::addi(0x10, 0x10, BLOB_ID_SIZE),
            // decrement the loop counter
            op::subi(0x13, 0x13, 1),
            // Jump backwards (3+1) instructions if the counter has not reached 0
            op::jnzb(0x13, RegId::ZERO, 3),
            // 2. Jump into the memory where the contract is loaded
            // what follows is called _jmp_mem by the sway compiler
            // subtract the address contained in IS because jmp will add it back
            op::sub(0x16, 0x16, RegId::IS),
            // jmp will multiply by 4 so we need to divide to cancel that out
            op::divi(0x16, 0x16, 4),
            // jump to the start of the contract we loaded
            op::jmp(0x16),
        ]
    };

    let num_of_instructions = u16::try_from(get_instructions(0, 0).len())
        .expect("to never have more than u16::MAX instructions");

    let num_of_blobs = u32::try_from(blob_ids.len()).map_err(|_| {
        error!(
            Other,
            "the number of blobs ({}) exceeds the maximum number of blobs supported: {}",
            blob_ids.len(),
            u32::MAX
        )
    })?;

    let instruction_bytes = get_instructions(num_of_instructions, num_of_blobs)
        .into_iter()
        .flat_map(|instruction| instruction.to_bytes());

    let blob_bytes = blob_ids.iter().flatten().copied();

    Ok(instruction_bytes.chain(blob_bytes).collect())
}

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

    fn compute_roots(&self) -> (ContractId, Bytes32, Bytes32) {
        compute_contract_id_and_state_root(&self.code(), &self.salt, &self.storage_slots)
    }

    /// This creates a loader contract for the code found in `blobs`. Deploying this contract
    /// happens in two stages:
    /// 1. the blobs are uploaded
    /// 2. the loader contract is deployed
    /// The loader contract, when executed, will load all the given blobs into memory and delegate the call to the original contract code contained in the blobs.
    pub fn loader_for_blobs(
        blobs: Vec<Blob>,
        salt: Salt,
        storage_slots: Vec<StorageSlot>,
    ) -> Result<Self> {
        if blobs.is_empty() {
            return Err(error!(Other, "must provide at least one blob"));
        }

        let idx_of_last_blob = blobs.len().saturating_sub(1);
        let idx_of_offender = blobs.iter().enumerate().find_map(|(idx, blob)| {
            (blob.data.len() % WORD_SIZE != 0 && idx != idx_of_last_blob).then_some(idx)
        });

        if let Some(idx) = idx_of_offender {
            return Err(error!(
                Other,
                "blob {}/{} has a size of {} bytes, which is not a multiple of {WORD_SIZE}",
                idx.saturating_add(1),
                blobs.len(),
                blobs[idx].data.len()
            ));
        }

        let ids = blobs.iter().map(|blob| blob.id()).collect::<Vec<_>>();

        // validate that the loader contract can be created
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

            let mut tb = BlobTransactionBuilder::default()
                .with_blob(blob)
                .with_tx_policies(tx_policies)
                .with_max_fee_estimation_tolerance(0.05);

            account.adjust_for_fee(&mut tb, 0).await?;
            account.add_witnesses(&mut tb)?;

            let tx = tb.build(provider).await?;

            let tx_status_response = provider.send_transaction_and_await_commit(tx).await;

            match tx_status_response {
                Ok(tx_status_response) => {
                    tx_status_response.check(None)?;
                }
                Err(err) => {
                    if !err
                        .to_string()
                        .contains("Execution error: BlobIdAlreadyUploaded")
                    {
                        return Err(err);
                    }
                }
            }

            already_uploaded.insert(id);
        }

        Contract::loader_for_blob_ids(all_blob_ids, self.salt, self.storage_slots)
    }

    pub async fn deploy(
        self,
        account: &impl Account,
        tx_policies: TxPolicies,
    ) -> Result<Bech32ContractId> {
        self.upload_blobs(account, tx_policies)
            .await?
            .deploy(account, tx_policies)
            .await
    }

    pub fn revert_to_regular(self) -> Contract<Regular> {
        let code = self
            .code
            .as_blobs
            .blobs
            .into_iter()
            .flat_map(|blob| blob.data)
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

    /// The contract code has been uploaded in blobs with [`BlobId`]s specified in `blob_ids`. This will create a loader
    /// contract that, when deployed and executed, will load all the specified blobs into memory and delegate the call to the code contained in the blobs.
    pub fn loader_for_blob_ids(
        blob_ids: Vec<BlobId>,
        salt: Salt,
        storage_slots: Vec<StorageSlot>,
    ) -> Result<Self> {
        if blob_ids.is_empty() {
            return Err(error!(Other, "must provide at least one blob"));
        }

        // validate that the loader contract can be created
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

    pub async fn deploy(
        self,
        account: &impl Account,
        tx_policies: TxPolicies,
    ) -> Result<Bech32ContractId> {
        Contract::regular(self.code(), self.salt, self.storage_slots)
            .deploy(account, tx_policies)
            .await
    }
}
