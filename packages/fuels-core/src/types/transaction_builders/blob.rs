use std::{fmt::Debug, iter::repeat, sync::Arc};

use async_trait::async_trait;
use fuel_core_client::client::types::assemble_tx::RequiredBalance;
use fuel_crypto::{Message as CryptoMessage, Signature};
use fuel_tx::{
    BlobIdExt, Chargeable, ConsensusParameters, Input as FuelInput, Output,
    Transaction as FuelTransaction, UniqueIdentifier, Witness,
    field::{Inputs, Policies as PoliciesField, Witnesses},
    input::{
        coin::CoinSigned,
        message::{MessageCoinSigned, MessageDataSigned},
    },
    policies::{Policies, PolicyType},
};
use fuel_types::bytes::padded_len_usize;
use itertools::Itertools;

use super::{
    BuildableTransaction, GAS_ESTIMATION_BLOCK_HORIZON, Strategy, TransactionBuilder,
    UnresolvedWitnessIndexes, generate_missing_witnesses, impl_tx_builder_trait,
    resolve_fuel_inputs,
};
use crate::{
    constants::SIGNATURE_WITNESS_SIZE,
    traits::Signer,
    types::{
        DryRunner,
        errors::{Result, error, error_transaction},
        input::Input,
        transaction::{BlobTransaction, EstimablePredicates, Transaction, TxPolicies},
    },
    utils::{calculate_witnesses_size, sealed},
};

#[derive(Default, Clone, Debug, PartialEq)]
pub struct Blob {
    data: Vec<u8>,
}

pub type BlobId = [u8; 32];

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

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn id(&self) -> BlobId {
        fuel_tx::BlobId::compute(&self.data).into()
    }

    pub fn bytes(&self) -> &[u8] {
        self.data.as_slice()
    }

    fn as_blob_body(&self, witness_index: u16) -> fuel_tx::BlobBody {
        fuel_tx::BlobBody {
            id: self.id().into(),
            witness_index,
        }
    }
}

impl From<Blob> for Vec<u8> {
    fn from(value: Blob) -> Self {
        value.data
    }
}

impl From<Blob> for fuel_tx::Witness {
    fn from(blob: Blob) -> Self {
        blob.data.into()
    }
}

#[derive(Debug, Clone)]
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
    unresolved_signers: Vec<Arc<dyn Signer + Send + Sync>>,
    enable_burn: bool,
}

impl Default for BlobTransactionBuilder {
    fn default() -> Self {
        Self {
            inputs: Default::default(),
            outputs: Default::default(),
            witnesses: Default::default(),
            tx_policies: Default::default(),
            gas_price_estimation_block_horizon: GAS_ESTIMATION_BLOCK_HORIZON,
            max_fee_estimation_tolerance: Default::default(),
            build_strategy: Default::default(),
            blob: Default::default(),
            unresolved_witness_indexes: Default::default(),
            unresolved_signers: Default::default(),
            enable_burn: false,
        }
    }
}
impl_tx_builder_trait!(BlobTransactionBuilder, BlobTransaction);

impl BlobTransactionBuilder {
    /// Calculates the maximum possible blob size by determining the remaining space available in the current transaction before it reaches the maximum allowed size.
    /// Note: This calculation only considers the transaction size limit and does not account for the maximum gas per transaction.
    pub async fn estimate_max_blob_size(&self, provider: &impl DryRunner) -> Result<usize> {
        let mut tb = self.clone();
        tb.blob = Blob::new(vec![]);

        let tx = tb
            .with_build_strategy(Strategy::NoSignatures)
            .build(provider)
            .await?;

        let current_tx_size = tx.size();
        let max_tx_size = usize::try_from(
            provider
                .consensus_parameters()
                .await?
                .tx_params()
                .max_size(),
        )
        .unwrap_or(usize::MAX);

        Ok(max_tx_size.saturating_sub(current_tx_size))
    }

    pub async fn build(mut self, provider: impl DryRunner) -> Result<BlobTransaction> {
        let consensus_parameters = provider.consensus_parameters().await?;
        self.intercept_burn(consensus_parameters.base_asset_id())?;

        let is_using_predicates = self.is_using_predicates();

        let tx = match self.build_strategy {
            Strategy::Complete => self.resolve_fuel_tx(&provider).await?,
            Strategy::NoSignatures => {
                self.set_witness_indexes();
                self.unresolved_signers = Default::default();
                self.resolve_fuel_tx(&provider).await?
            }
            Strategy::AssembleTx {
                ref required_balances,
                fee_index,
            } => {
                let required_balances = required_balances.clone(); //TODO: Fix this
                self.assemble_tx(
                    required_balances,
                    fee_index,
                    &consensus_parameters,
                    provider,
                )
                .await?
            }
        };

        Ok(BlobTransaction {
            is_using_predicates,
            tx,
        })
    }

    async fn assemble_tx(
        mut self,
        required_balances: Vec<RequiredBalance>,
        fee_index: u16,
        consensus_parameters: &ConsensusParameters,
        dry_runner: impl DryRunner,
    ) -> Result<fuel_tx::Blob> {
        let free_witness_index = self.num_witnesses()?;
        let body = self.blob.as_blob_body(free_witness_index);

        let blob_witness = std::mem::take(&mut self.blob).into();
        self.witnesses_mut().push(blob_witness);

        let num_witnesses = self.num_witnesses()?;
        let policies = self.generate_fuel_policies_assemble();

        let mut tx = FuelTransaction::blob(
            body,
            policies,
            resolve_fuel_inputs(self.inputs, num_witnesses, &self.unresolved_witness_indexes)?,
            self.outputs,
            self.witnesses,
        );

        if let Some(max_fee) = self.tx_policies.max_fee() {
            tx.policies_mut().set(PolicyType::MaxFee, Some(max_fee));
        };

        let fuel_tx = FuelTransaction::Blob(tx);
        let mut tx = dry_runner
            .assemble_tx(
                &fuel_tx,
                self.gas_price_estimation_block_horizon,
                required_balances,
                fee_index,
                None,
                true,
                None,
            )
            .await?
            .transaction
            .as_blob()
            .expect("is upgrade")
            .clone(); //TODO: do not clone

        let id = tx.id(&consensus_parameters.chain_id());

        for signer in &self.unresolved_signers {
            let message = CryptoMessage::from_bytes(*id);
            let signature = signer.sign(message).await?;
            let address = signer.address().into();

            let witness_indexes = tx
                .inputs()
                .iter()
                .filter_map(|input| match input {
                    FuelInput::CoinSigned(CoinSigned {
                        owner,
                        witness_index,
                        ..
                    })
                    | FuelInput::MessageCoinSigned(MessageCoinSigned {
                        recipient: owner,
                        witness_index,
                        ..
                    })
                    | FuelInput::MessageDataSigned(MessageDataSigned {
                        recipient: owner,
                        witness_index,
                        ..
                    }) if owner == &address => Some(*witness_index as usize),
                    _ => None,
                })
                .sorted()
                .dedup()
                .collect_vec();

            for w in witness_indexes {
                if let Some(w) = tx.witnesses_mut().get_mut(w) {
                    *w = signature.as_ref().into();
                }
            }
        }

        Ok(tx)
    }

    async fn resolve_fuel_tx(mut self, provider: &impl DryRunner) -> Result<fuel_tx::Blob> {
        let chain_id = provider.consensus_parameters().await?.chain_id();

        let free_witness_index = self.num_witnesses()?;
        let body = self.blob.as_blob_body(free_witness_index);

        let blob_witness = std::mem::take(&mut self.blob).into();
        self.witnesses_mut().push(blob_witness);

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

    pub fn with_blob(mut self, blob: Blob) -> Self {
        self.blob = blob;
        self
    }

    pub fn with_max_fee_estimation_tolerance(mut self, max_fee_estimation_tolerance: f32) -> Self {
        self.max_fee_estimation_tolerance = max_fee_estimation_tolerance;
        self
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
