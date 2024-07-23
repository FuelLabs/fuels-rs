use std::{
    collections::HashMap,
    fmt::{Debug, Formatter},
    iter::repeat,
};

use async_trait::async_trait;
use fuel_asm::{op, GTFArgs, RegId};
use fuel_crypto::{Hasher, Message as CryptoMessage, Signature};
use fuel_tx::{
    field::{Outputs, Policies as PoliciesField, ScriptGasLimit, Witnesses},
    policies::{Policies, PolicyType},
    BlobId, BlobIdExt, Chargeable, ConsensusParameters, Create, Input as FuelInput, Output, Script,
    StorageSlot, Transaction as FuelTransaction, TransactionFee, TxPointer, UniqueIdentifier,
    Upgrade, Upload, UploadBody, Witness,
};
pub use fuel_tx::{UpgradePurpose, UploadSubsection};
use fuel_types::{bytes::padded_len_usize, Bytes32, Salt};
use itertools::Itertools;

use crate::{
    constants::{SIGNATURE_WITNESS_SIZE, WORD_SIZE},
    traits::Signer,
    types::{
        bech32::Bech32Address,
        coin::Coin,
        coin_type::CoinType,
        errors::{error, error_transaction, Result},
        input::Input,
        message::Message,
        transaction::{
            BlobTransaction, CreateTransaction, EstimablePredicates, ScriptTransaction,
            Transaction, TxPolicies, UpgradeTransaction, UploadTransaction,
        },
        Address, AssetId, ContractId, DryRunner,
    },
    utils::{calculate_witnesses_size, sealed},
};

use super::{
    generate_missing_witnesses, impl_tx_builder_trait, resolve_fuel_inputs, BuildableTransaction,
    Strategy, UnresolvedWitnessIndexes,
};

#[derive(Default, Clone, Debug, PartialEq)]
pub struct Blob {
    pub data: Vec<u8>,
}

impl From<Vec<u8>> for Blob {
    fn from(data: Vec<u8>) -> Self {
        Self { data }
    }
}

impl AsRef<[u8]> for Blob {
    fn as_ref(&self) -> &[u8] {
        &self.data
    }
}

impl Blob {
    pub fn new(data: Vec<u8>) -> Self {
        Self { data }
    }

    pub fn id(&self) -> [u8; 32] {
        BlobId::compute(&self.data).into()
    }

    fn as_blob_body(&self, witness_index: u16) -> fuel_tx::BlobBody {
        fuel_tx::BlobBody {
            id: self.id().into(),
            witness_index,
        }
    }
}

impl From<Blob> for fuel_tx::Witness {
    fn from(blob: Blob) -> Self {
        blob.data.into()
    }
}

#[derive(Default)]
pub struct BlobTransactionBuilder {
    pub inputs: Vec<Input>,
    pub outputs: Vec<Output>,
    pub witnesses: Vec<Witness>,
    pub tx_policies: TxPolicies,
    pub gas_price_estimation_block_horizon: u32,
    pub max_fee_estimation_tolerance: f32,
    pub build_strategy: Strategy,
    pub blob: Blob,
    unresolved_witness_indexes: UnresolvedWitnessIndexes,
    unresolved_signers: Vec<Box<dyn Signer + Send + Sync>>,
}
impl_tx_builder_trait!(BlobTransactionBuilder, BlobTransaction);

impl BlobTransactionBuilder {
    pub async fn build(mut self, provider: impl DryRunner) -> Result<BlobTransaction> {
        let is_using_predicates = self.is_using_predicates();

        let tx = match self.build_strategy {
            Strategy::Complete => self.resolve_fuel_tx(&provider).await?,
            Strategy::NoSignatures => {
                self.set_witness_indexes();
                self.unresolved_signers = Default::default();
                self.resolve_fuel_tx(&provider).await?
            }
        };

        Ok(BlobTransaction {
            is_using_predicates,
            tx,
        })
    }

    fn clone_without_signers(&self) -> Self {
        Self {
            inputs: self.inputs.clone(),
            outputs: self.outputs.clone(),
            witnesses: self.witnesses.clone(),
            tx_policies: self.tx_policies,
            unresolved_witness_indexes: self.unresolved_witness_indexes.clone(),
            unresolved_signers: Default::default(),
            gas_price_estimation_block_horizon: self.gas_price_estimation_block_horizon,
            max_fee_estimation_tolerance: self.max_fee_estimation_tolerance,
            build_strategy: self.build_strategy.clone(),
            blob: self.blob.clone(),
        }
    }

    async fn resolve_fuel_tx(mut self, provider: &impl DryRunner) -> Result<fuel_tx::Blob> {
        let chain_id = provider.consensus_parameters().chain_id();

        // TODO: segfault
        let blob_witness_index = self.witnesses.len() as u16;
        let body = self.blob.as_blob_body(blob_witness_index);
        let blob_witness = std::mem::take(&mut self.blob).into();
        self.witnesses.push(blob_witness);

        let num_witnesses = self.num_witnesses()?;
        let policies = self.generate_fuel_policies()?;
        let is_using_predicates = self.is_using_predicates();

        let mut tx = FuelTransaction::blob(
            body,
            policies,
            resolve_fuel_inputs(self.inputs, num_witnesses, &self.unresolved_witness_indexes)?,
            self.outputs,
            self.witnesses,
        );

        if let Some(max_fee) = self.tx_policies.max_fee() {
            tx.policies_mut().set(PolicyType::MaxFee, Some(max_fee));
        } else {
            Self::set_max_fee_policy(
                &mut tx,
                &provider,
                self.gas_price_estimation_block_horizon,
                is_using_predicates,
                self.max_fee_estimation_tolerance,
            )
            .await?;
        }

        let signatures =
            generate_missing_witnesses(tx.id(&chain_id), &self.unresolved_signers).await?;
        tx.witnesses_mut().extend(signatures);

        Ok(tx)
    }
}

impl sealed::Sealed for BlobTransactionBuilder {}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl BuildableTransaction for BlobTransactionBuilder {
    type TxType = BlobTransaction;
    type Strategy = Strategy;

    fn with_build_strategy(mut self, strategy: Self::Strategy) -> Self {
        self.build_strategy = strategy;
        self
    }

    async fn build(self, provider: impl DryRunner) -> Result<Self::TxType> {
        BlobTransactionBuilder::build(self, provider).await
    }
}
