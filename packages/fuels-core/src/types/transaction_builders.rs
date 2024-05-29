#![cfg(feature = "std")]

use std::{
    collections::HashMap,
    fmt::{Debug, Formatter},
    iter::repeat,
};

use async_trait::async_trait;
use fuel_asm::{op, GTFArgs, RegId};
use fuel_crypto::{Hasher, Message as CryptoMessage, Signature};
use fuel_tx::{
    field::{Inputs, Policies as PoliciesField, ScriptGasLimit, WitnessLimit, Witnesses},
    input::coin::{CoinPredicate, CoinSigned},
    policies::{Policies, PolicyType},
    Chargeable, ConsensusParameters, Create, Input as FuelInput, Output, Script, StorageSlot,
    Transaction as FuelTransaction, TransactionFee, TxPointer, UniqueIdentifier, Upgrade, Upload,
    UploadBody, Witness,
};

pub use fuel_tx::UpgradePurpose;
pub use fuel_tx::UploadSubsection;
use fuel_types::{bytes::padded_len_usize, Bytes32, Salt};
use itertools::Itertools;

use crate::{
    constants::{SIGNATURE_WITNESS_SIZE, WITNESS_STATIC_SIZE, WORD_SIZE},
    traits::Signer,
    types::{
        bech32::Bech32Address,
        coin::Coin,
        coin_type::CoinType,
        errors::{error, error_transaction, Result},
        input::Input,
        message::Message,
        transaction::{
            CreateTransaction, EstimablePredicates, ScriptTransaction, Transaction, TxPolicies,
            UpgradeTransaction, UploadTransaction,
        },
        Address, AssetId, ContractId,
    },
    utils::{calculate_witnesses_size, sealed},
};

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait DryRunner: Send + Sync {
    async fn dry_run_and_get_used_gas(&self, tx: FuelTransaction, tolerance: f32) -> Result<u64>;
    async fn estimate_gas_price(&self, block_horizon: u32) -> Result<u64>;
    fn consensus_parameters(&self) -> &ConsensusParameters;
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<T: DryRunner> DryRunner for &T {
    async fn dry_run_and_get_used_gas(&self, tx: FuelTransaction, tolerance: f32) -> Result<u64> {
        (*self).dry_run_and_get_used_gas(tx, tolerance).await
    }

    async fn estimate_gas_price(&self, block_horizon: u32) -> Result<u64> {
        (*self).estimate_gas_price(block_horizon).await
    }

    fn consensus_parameters(&self) -> &ConsensusParameters {
        (*self).consensus_parameters()
    }
}

#[derive(Debug, Clone, Default)]
struct UnresolvedWitnessIndexes {
    owner_to_idx_offset: HashMap<Bech32Address, u64>,
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait BuildableTransaction: sealed::Sealed {
    type TxType: Transaction;

    async fn build(self, provider: impl DryRunner) -> Result<Self::TxType>;

    /// Building without signatures will set the witness indexes of signed coins in the
    /// order as they appear in the inputs. Multiple coins with the same owner will have
    /// the same witness index. Make sure you sign the built transaction in the expected order.
    async fn build_without_signatures(self, provider: impl DryRunner) -> Result<Self::TxType>;
}

impl sealed::Sealed for ScriptTransactionBuilder {}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl BuildableTransaction for ScriptTransactionBuilder {
    type TxType = ScriptTransaction;

    async fn build(self, provider: impl DryRunner) -> Result<Self::TxType> {
        self.build(provider).await
    }

    async fn build_without_signatures(mut self, provider: impl DryRunner) -> Result<Self::TxType> {
        self.set_witness_indexes();
        self.unresolved_signers = Default::default();

        self.build(provider).await
    }
}

impl sealed::Sealed for CreateTransactionBuilder {}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl BuildableTransaction for CreateTransactionBuilder {
    type TxType = CreateTransaction;

    async fn build(self, provider: impl DryRunner) -> Result<Self::TxType> {
        self.build(provider).await
    }

    async fn build_without_signatures(mut self, provider: impl DryRunner) -> Result<Self::TxType> {
        self.set_witness_indexes();
        self.unresolved_signers = Default::default();

        self.build(provider).await
    }
}

impl sealed::Sealed for UploadTransactionBuilder {}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl BuildableTransaction for UploadTransactionBuilder {
    type TxType = UploadTransaction;

    async fn build(self, provider: impl DryRunner) -> Result<Self::TxType> {
        self.build(provider).await
    }

    async fn build_without_signatures(mut self, provider: impl DryRunner) -> Result<Self::TxType> {
        self.set_witness_indexes();
        self.unresolved_signers = Default::default();

        self.build(provider).await
    }
}

impl sealed::Sealed for UpgradeTransactionBuilder {}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl BuildableTransaction for UpgradeTransactionBuilder {
    type TxType = UpgradeTransaction;

    async fn build(self, provider: impl DryRunner) -> Result<Self::TxType> {
        self.build(provider).await
    }

    async fn build_without_signatures(mut self, provider: impl DryRunner) -> Result<Self::TxType> {
        self.set_witness_indexes();
        self.unresolved_signers = Default::default();

        self.build(provider).await
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait TransactionBuilder: BuildableTransaction + Send + sealed::Sealed {
    type TxType: Transaction;

    fn add_signer(&mut self, signer: impl Signer + Send + Sync) -> Result<&mut Self>;
    async fn fee_checked_from_tx(&self, provider: impl DryRunner)
        -> Result<Option<TransactionFee>>;
    fn with_tx_policies(self, tx_policies: TxPolicies) -> Self;
    fn with_inputs(self, inputs: Vec<Input>) -> Self;
    fn with_outputs(self, outputs: Vec<Output>) -> Self;
    fn with_witnesses(self, witnesses: Vec<Witness>) -> Self;
    fn inputs(&self) -> &Vec<Input>;
    fn inputs_mut(&mut self) -> &mut Vec<Input>;
    fn outputs(&self) -> &Vec<Output>;
    fn outputs_mut(&mut self) -> &mut Vec<Output>;
    fn witnesses(&self) -> &Vec<Witness>;
    fn witnesses_mut(&mut self) -> &mut Vec<Witness>;
    fn with_estimation_horizon(self, block_horizon: u32) -> Self;
}

macro_rules! impl_tx_trait {
    ($ty: ty, $tx_ty: ident) => {
        #[cfg_attr(not(target_arch = "wasm32"), async_trait)]
        impl TransactionBuilder for $ty {
            type TxType = $tx_ty;

            fn add_signer(&mut self, signer: impl Signer + Send + Sync) -> Result<&mut Self> {
                let address = signer.address();
                if self
                    .unresolved_witness_indexes
                    .owner_to_idx_offset
                    .contains_key(address)
                {
                    return Err(error_transaction!(
                        Builder,
                        "already added `Signer` with address: `{address}`"
                    ));
                }

                let index_offset = self.unresolved_signers.len() as u64;
                self.unresolved_witness_indexes
                    .owner_to_idx_offset
                    .insert(address.clone(), index_offset);
                self.unresolved_signers.push(Box::new(signer));

                Ok(self)
            }

            async fn fee_checked_from_tx(
                &self,
                provider: impl DryRunner,
            ) -> Result<Option<TransactionFee>> {
                let mut fee_estimation_tb = self.clone_without_signers();

                // Add a temporary witness for every `Signer` to include them in the fee
                // estimation.
                let witness: Witness = Signature::default().as_ref().into();
                fee_estimation_tb
                    .witnesses_mut()
                    .extend(repeat(witness).take(self.unresolved_signers.len()));

                let mut tx =
                    BuildableTransaction::build_without_signatures(fee_estimation_tb, &provider)
                        .await?;

                let consensus_parameters = provider.consensus_parameters();

                if tx.is_using_predicates() {
                    tx.estimate_predicates(consensus_parameters)?;
                }

                Ok(TransactionFee::checked_from_tx(
                    &consensus_parameters.gas_costs(),
                    &consensus_parameters.fee_params(),
                    &tx.tx,
                    provider
                        .estimate_gas_price(self.gas_price_estimation_block_horizon)
                        .await?,
                ))
            }

            fn with_tx_policies(mut self, tx_policies: TxPolicies) -> Self {
                self.tx_policies = tx_policies;

                self
            }

            fn with_inputs(mut self, inputs: Vec<Input>) -> Self {
                self.inputs = inputs;
                self
            }

            fn with_outputs(mut self, outputs: Vec<Output>) -> Self {
                self.outputs = outputs;
                self
            }

            fn with_witnesses(mut self, witnesses: Vec<Witness>) -> Self {
                self.witnesses = witnesses;
                self
            }

            fn inputs(&self) -> &Vec<Input> {
                self.inputs.as_ref()
            }

            fn inputs_mut(&mut self) -> &mut Vec<Input> {
                &mut self.inputs
            }

            fn outputs(&self) -> &Vec<Output> {
                self.outputs.as_ref()
            }

            fn outputs_mut(&mut self) -> &mut Vec<Output> {
                &mut self.outputs
            }

            fn witnesses(&self) -> &Vec<Witness> {
                self.witnesses.as_ref()
            }

            fn witnesses_mut(&mut self) -> &mut Vec<Witness> {
                &mut self.witnesses
            }

            fn with_estimation_horizon(mut self, block_horizon: u32) -> Self {
                self.gas_price_estimation_block_horizon = block_horizon;

                self
            }
        }

        impl $ty {
            fn set_witness_indexes(&mut self) {
                self.unresolved_witness_indexes.owner_to_idx_offset = self
                    .inputs()
                    .iter()
                    .filter_map(|input| match input {
                        Input::ResourceSigned { resource } => Some(resource.owner()),
                        _ => None,
                    })
                    .unique()
                    .cloned()
                    .enumerate()
                    .map(|(idx, owner)| (owner, idx as u64))
                    .collect();
            }

            fn generate_fuel_policies(&self) -> Result<Policies> {
                let witness_limit = match self.tx_policies.witness_limit() {
                    Some(limit) => limit,
                    None => self.calculate_witnesses_size()?,
                };
                let mut policies = Policies::default().with_witness_limit(witness_limit);

                // `MaxFee` set to `tip` or `0` for `dry_run`
                policies.set(PolicyType::MaxFee, self.tx_policies.tip().or(Some(0)));
                policies.set(PolicyType::Maturity, self.tx_policies.maturity());
                policies.set(PolicyType::Tip, self.tx_policies.tip());

                Ok(policies)
            }

            fn is_using_predicates(&self) -> bool {
                self.inputs()
                    .iter()
                    .any(|input| matches!(input, Input::ResourcePredicate { .. }))
            }

            fn num_witnesses(&self) -> Result<u16> {
                let num_witnesses = self.witnesses().len();

                if num_witnesses + self.unresolved_signers.len() > u16::MAX as usize {
                    return Err(error_transaction!(
                        Builder,
                        "tx exceeds maximum number of witnesses"
                    ));
                }

                Ok(num_witnesses as u16)
            }

            fn calculate_witnesses_size(&self) -> Result<u64> {
                let witnesses_size = calculate_witnesses_size(&self.witnesses);
                let signature_size = SIGNATURE_WITNESS_SIZE
                    * self.unresolved_witness_indexes.owner_to_idx_offset.len();

                let padded_len = padded_len_usize(witnesses_size + signature_size)
                    .ok_or_else(|| error!(Other, "witnesses size overflow"))?;
                Ok(padded_len as u64)
            }

            async fn set_max_fee_policy<T: PoliciesField + Chargeable>(
                tx: &mut T,
                provider: impl DryRunner,
                block_horizon: u32,
            ) -> Result<()> {
                let gas_price = provider.estimate_gas_price(block_horizon).await?;
                let consensus_parameters = provider.consensus_parameters();

                let tx_fee = TransactionFee::checked_from_tx(
                    &consensus_parameters.gas_costs(),
                    consensus_parameters.fee_params(),
                    tx,
                    gas_price,
                )
                .ok_or(error_transaction!(
                    Other,
                    "error calculating `TransactionFee` in `TransactionBuilder`"
                ))?;

                tx.policies_mut()
                    .set(PolicyType::MaxFee, Some(tx_fee.max_fee()));

                Ok(())
            }
        }
    };
}

impl Debug for dyn Signer + Send + Sync {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Signer")
            .field("address", &self.address())
            .finish()
    }
}

#[derive(Debug, Default)]
pub struct ScriptTransactionBuilder {
    pub script: Vec<u8>,
    pub script_data: Vec<u8>,
    pub inputs: Vec<Input>,
    pub outputs: Vec<Output>,
    pub witnesses: Vec<Witness>,
    pub tx_policies: TxPolicies,
    pub gas_estimation_tolerance: f32,
    pub gas_price_estimation_block_horizon: u32,
    unresolved_witness_indexes: UnresolvedWitnessIndexes,
    unresolved_signers: Vec<Box<dyn Signer + Send + Sync>>,
}

#[derive(Default)]
pub struct CreateTransactionBuilder {
    pub bytecode_length: u64,
    pub bytecode_witness_index: u16,
    pub storage_slots: Vec<StorageSlot>,
    pub inputs: Vec<Input>,
    pub outputs: Vec<Output>,
    pub witnesses: Vec<Witness>,
    pub tx_policies: TxPolicies,
    pub salt: Salt,
    pub gas_price_estimation_block_horizon: u32,
    unresolved_witness_indexes: UnresolvedWitnessIndexes,
    unresolved_signers: Vec<Box<dyn Signer + Send + Sync>>,
}

#[derive(Default)]
pub struct UploadTransactionBuilder {
    /// The root of the Merkle tree is created over the bytecode.
    pub root: Bytes32,
    /// The witness index of the subsection of the bytecode.
    pub witness_index: u16,
    /// The index of the subsection of the bytecode.
    pub subsection_index: u16,
    /// The total number of subsections on which bytecode was divided.
    pub subsections_number: u16,
    /// The proof set helps to verify the connection of the subsection to the `root`.
    pub proof_set: Vec<Bytes32>,
    pub inputs: Vec<Input>,
    pub outputs: Vec<Output>,
    pub witnesses: Vec<Witness>,
    pub tx_policies: TxPolicies,
    pub gas_price_estimation_block_horizon: u32,
    unresolved_witness_indexes: UnresolvedWitnessIndexes,
    unresolved_signers: Vec<Box<dyn Signer + Send + Sync>>,
}

pub struct UpgradeTransactionBuilder {
    /// The purpose of the upgrade.
    pub purpose: UpgradePurpose,
    pub inputs: Vec<Input>,
    pub outputs: Vec<Output>,
    pub witnesses: Vec<Witness>,
    pub tx_policies: TxPolicies,
    pub gas_price_estimation_block_horizon: u32,
    unresolved_witness_indexes: UnresolvedWitnessIndexes,
    unresolved_signers: Vec<Box<dyn Signer + Send + Sync>>,
}

impl Default for UpgradeTransactionBuilder {
    fn default() -> Self {
        Self {
            purpose: UpgradePurpose::StateTransition {
                root: Default::default(),
            },
            inputs: Default::default(),
            outputs: Default::default(),
            witnesses: Default::default(),
            tx_policies: Default::default(),
            gas_price_estimation_block_horizon: Default::default(),
            unresolved_witness_indexes: Default::default(),
            unresolved_signers: Default::default(),
        }
    }
}

impl_tx_trait!(ScriptTransactionBuilder, ScriptTransaction);
impl_tx_trait!(CreateTransactionBuilder, CreateTransaction);
impl_tx_trait!(UploadTransactionBuilder, UploadTransaction);
impl_tx_trait!(UpgradeTransactionBuilder, UpgradeTransaction);

impl ScriptTransactionBuilder {
    async fn build(self, provider: impl DryRunner) -> Result<ScriptTransaction> {
        Ok(ScriptTransaction {
            is_using_predicates: self.is_using_predicates(),
            tx: self.resolve_fuel_tx(&provider).await?,
        })
    }

    // When dry running a tx with `utxo_validation` off, the node will not validate signatures.
    // However, the node will check if the right number of witnesses is present.
    // This function will create witnesses from a default `Signature` such that the total length matches the expected one.
    // Using a `Signature` ensures that the calculated fee includes the fee generated by the witnesses.
    fn create_dry_run_witnesses(&self, num_witnesses: u16) -> Vec<Witness> {
        let unresolved_witnesses_len = self.unresolved_witness_indexes.owner_to_idx_offset.len();
        let witness: Witness = Signature::default().as_ref().into();
        repeat(witness)
            .take(num_witnesses as usize + unresolved_witnesses_len)
            .collect()
    }

    async fn resolve_fuel_tx(self, provider: impl DryRunner) -> Result<Script> {
        let num_witnesses = self.num_witnesses()?;
        let policies = self.generate_fuel_policies()?;

        let has_no_code = self.script.is_empty();
        let dry_run_witnesses = self.create_dry_run_witnesses(num_witnesses);
        let mut tx = FuelTransaction::script(
            0, // default value - will be overwritten
            self.script,
            self.script_data,
            policies,
            resolve_fuel_inputs(self.inputs, num_witnesses, &self.unresolved_witness_indexes)?,
            self.outputs,
            dry_run_witnesses,
        );

        let script_gas_limit = if has_no_code {
            0
        } else if let Some(gas_limit) = self.tx_policies.script_gas_limit() {
            // Use the user defined value even if it makes the transaction revert.
            gas_limit
        } else {
            Self::run_estimation(tx.clone(), &provider, self.gas_estimation_tolerance).await?
        };

        *tx.script_gas_limit_mut() = script_gas_limit;

        Self::set_max_fee_policy(&mut tx, &provider, self.gas_price_estimation_block_horizon)
            .await?;

        let missing_witnesses = generate_missing_witnesses(
            tx.id(&provider.consensus_parameters().chain_id()),
            &self.unresolved_signers,
        )
        .await?;
        *tx.witnesses_mut() = [self.witnesses, missing_witnesses].concat();

        Ok(tx)
    }

    async fn run_estimation(
        mut tx: fuel_tx::Script,
        provider: impl DryRunner,
        tolerance: f32,
    ) -> Result<u64> {
        let consensus_params = provider.consensus_parameters();
        if let Some(fake_input) =
            needs_fake_base_input(tx.inputs(), consensus_params.base_asset_id())
        {
            tx.inputs_mut().push(fake_input);

            // Add an empty `Witness` for the `coin_signed` we just added
            tx.witnesses_mut().push(Default::default());
            tx.set_witness_limit(tx.witness_limit() + WITNESS_STATIC_SIZE as u64);
        }

        let max_gas = tx.max_gas(consensus_params.gas_costs(), consensus_params.fee_params()) + 1;
        *tx.script_gas_limit_mut() = consensus_params.tx_params().max_gas_per_tx() - max_gas;

        provider
            .dry_run_and_get_used_gas(tx.into(), tolerance)
            .await
    }

    pub fn with_script(mut self, script: Vec<u8>) -> Self {
        self.script = script;
        self
    }

    pub fn with_script_data(mut self, script_data: Vec<u8>) -> Self {
        self.script_data = script_data;
        self
    }

    pub fn with_gas_estimation_tolerance(mut self, tolerance: f32) -> Self {
        self.gas_estimation_tolerance = tolerance;
        self
    }

    pub fn prepare_transfer(
        inputs: Vec<Input>,
        outputs: Vec<Output>,
        tx_policies: TxPolicies,
    ) -> Self {
        ScriptTransactionBuilder::default()
            .with_inputs(inputs)
            .with_outputs(outputs)
            .with_tx_policies(tx_policies)
    }

    /// Craft a transaction used to transfer funds to a contract.
    pub fn prepare_contract_transfer(
        to: ContractId,
        amount: u64,
        asset_id: AssetId,
        inputs: Vec<Input>,
        outputs: Vec<Output>,
        tx_policies: TxPolicies,
    ) -> Self {
        let script_data: Vec<u8> = [
            to.to_vec(),
            amount.to_be_bytes().to_vec(),
            asset_id.to_vec(),
        ]
        .into_iter()
        .flatten()
        .collect();

        // This script loads:
        //  - a pointer to the contract id,
        //  - the actual amount
        //  - a pointer to the asset id
        // into the registers 0x10, 0x12, 0x13
        // and calls the TR instruction
        let script = vec![
            op::gtf(0x10, 0x00, GTFArgs::ScriptData.into()),
            op::addi(0x11, 0x10, ContractId::LEN as u16),
            op::lw(0x12, 0x11, 0),
            op::addi(0x13, 0x11, WORD_SIZE as u16),
            op::tr(0x10, 0x12, 0x13),
            op::ret(RegId::ONE),
        ]
        .into_iter()
        .collect();

        ScriptTransactionBuilder::default()
            .with_script(script)
            .with_script_data(script_data)
            .with_inputs(inputs)
            .with_outputs(outputs)
            .with_tx_policies(tx_policies)
    }

    /// Craft a transaction used to transfer funds to the base chain.
    pub fn prepare_message_to_output(
        to: Address,
        amount: u64,
        inputs: Vec<Input>,
        tx_policies: TxPolicies,
        base_asset_id: AssetId,
    ) -> Self {
        let script_data: Vec<u8> = [to.to_vec(), amount.to_be_bytes().to_vec()]
            .into_iter()
            .flatten()
            .collect();

        // This script loads:
        //  - a pointer to the recipient address,
        //  - the amount
        // into the registers 0x10, 0x11
        // and calls the SMO instruction
        let script: Vec<u8> = vec![
            op::gtf(0x10, 0x00, GTFArgs::ScriptData.into()),
            op::addi(0x11, 0x10, Bytes32::LEN as u16),
            op::lw(0x11, 0x11, 0),
            op::smo(0x10, 0x00, 0x00, 0x11),
            op::ret(RegId::ONE),
        ]
        .into_iter()
        .collect();

        let outputs = vec![Output::change(to, 0, base_asset_id)];

        ScriptTransactionBuilder::default()
            .with_tx_policies(tx_policies)
            .with_script(script)
            .with_script_data(script_data)
            .with_inputs(inputs)
            .with_outputs(outputs)
    }

    fn clone_without_signers(&self) -> Self {
        Self {
            script: self.script.clone(),
            script_data: self.script_data.clone(),
            inputs: self.inputs.clone(),
            outputs: self.outputs.clone(),
            witnesses: self.witnesses.clone(),
            tx_policies: self.tx_policies,
            gas_estimation_tolerance: self.gas_estimation_tolerance,
            unresolved_witness_indexes: self.unresolved_witness_indexes.clone(),
            unresolved_signers: Default::default(),
            gas_price_estimation_block_horizon: self.gas_price_estimation_block_horizon,
        }
    }
}

fn needs_fake_base_input(inputs: &[FuelInput], base_asset_id: &AssetId) -> Option<fuel_tx::Input> {
    let has_base_asset = inputs.iter().any(|i| match i {
        FuelInput::CoinSigned(CoinSigned { asset_id, .. })
        | FuelInput::CoinPredicate(CoinPredicate { asset_id, .. })
            if asset_id == base_asset_id =>
        {
            true
        }
        FuelInput::MessageCoinSigned(_) | FuelInput::MessageCoinPredicate(_) => true,
        _ => false,
    });

    if has_base_asset {
        return None;
    }

    let unique_owners = inputs
        .iter()
        .filter_map(|input| match input {
            FuelInput::CoinSigned(CoinSigned { owner, .. })
            | FuelInput::CoinPredicate(CoinPredicate { owner, .. }) => Some(owner),
            _ => None,
        })
        .unique()
        .collect::<Vec<_>>();

    let fake_owner = if let [single_owner] = unique_owners.as_slice() {
        **single_owner
    } else {
        Default::default()
    };

    Some(FuelInput::coin_signed(
        Default::default(),
        fake_owner,
        1_000_000_000,
        Default::default(),
        TxPointer::default(),
        0,
    ))
}

impl CreateTransactionBuilder {
    pub async fn build(self, provider: impl DryRunner) -> Result<CreateTransaction> {
        Ok(CreateTransaction {
            is_using_predicates: self.is_using_predicates(),
            tx: self.resolve_fuel_tx(&provider).await?,
        })
    }

    async fn resolve_fuel_tx(self, provider: impl DryRunner) -> Result<Create> {
        let chain_id = provider.consensus_parameters().chain_id();
        let num_witnesses = self.num_witnesses()?;
        let policies = self.generate_fuel_policies()?;

        let mut tx = FuelTransaction::create(
            self.bytecode_witness_index,
            policies,
            self.salt,
            self.storage_slots,
            resolve_fuel_inputs(self.inputs, num_witnesses, &self.unresolved_witness_indexes)?,
            self.outputs,
            self.witnesses,
        );

        Self::set_max_fee_policy(&mut tx, provider, self.gas_price_estimation_block_horizon)
            .await?;

        let missing_witnesses =
            generate_missing_witnesses(tx.id(&chain_id), &self.unresolved_signers).await?;
        tx.witnesses_mut().extend(missing_witnesses);

        Ok(tx)
    }

    pub fn with_bytecode_length(mut self, bytecode_length: u64) -> Self {
        self.bytecode_length = bytecode_length;
        self
    }

    pub fn with_bytecode_witness_index(mut self, bytecode_witness_index: u16) -> Self {
        self.bytecode_witness_index = bytecode_witness_index;
        self
    }

    pub fn with_storage_slots(mut self, mut storage_slots: Vec<StorageSlot>) -> Self {
        // Storage slots have to be sorted otherwise we'd get a `TransactionCreateStorageSlotOrder`
        // error.
        storage_slots.sort();
        self.storage_slots = storage_slots;
        self
    }

    pub fn with_salt(mut self, salt: impl Into<Salt>) -> Self {
        self.salt = salt.into();
        self
    }

    pub fn prepare_contract_deployment(
        binary: Vec<u8>,
        contract_id: ContractId,
        state_root: Bytes32,
        salt: Salt,
        storage_slots: Vec<StorageSlot>,
        tx_policies: TxPolicies,
    ) -> Self {
        let bytecode_witness_index = 0;
        let outputs = vec![Output::contract_created(contract_id, state_root)];
        let witnesses = vec![binary.into()];

        CreateTransactionBuilder::default()
            .with_tx_policies(tx_policies)
            .with_bytecode_witness_index(bytecode_witness_index)
            .with_salt(salt)
            .with_storage_slots(storage_slots)
            .with_outputs(outputs)
            .with_witnesses(witnesses)
    }

    fn clone_without_signers(&self) -> Self {
        Self {
            bytecode_length: self.bytecode_length,
            bytecode_witness_index: self.bytecode_witness_index,
            storage_slots: self.storage_slots.clone(),
            inputs: self.inputs.clone(),
            outputs: self.outputs.clone(),
            witnesses: self.witnesses.clone(),
            tx_policies: self.tx_policies,
            salt: self.salt,
            unresolved_witness_indexes: self.unresolved_witness_indexes.clone(),
            unresolved_signers: Default::default(),
            gas_price_estimation_block_horizon: self.gas_price_estimation_block_horizon,
        }
    }
}

impl UploadTransactionBuilder {
    pub async fn build(self, provider: impl DryRunner) -> Result<UploadTransaction> {
        Ok(UploadTransaction {
            is_using_predicates: self.is_using_predicates(),
            tx: self.resolve_fuel_tx(&provider).await?,
        })
    }

    async fn resolve_fuel_tx(self, provider: impl DryRunner) -> Result<Upload> {
        let chain_id = provider.consensus_parameters().chain_id();
        let num_witnesses = self.num_witnesses()?;
        let policies = self.generate_fuel_policies()?;

        let mut tx = FuelTransaction::upload(
            UploadBody {
                root: self.root,
                witness_index: self.witness_index,
                subsection_index: self.subsection_index,
                subsections_number: self.subsections_number,
                proof_set: self.proof_set,
            },
            policies,
            resolve_fuel_inputs(self.inputs, num_witnesses, &self.unresolved_witness_indexes)?,
            self.outputs,
            self.witnesses,
        );

        Self::set_max_fee_policy(&mut tx, provider, self.gas_price_estimation_block_horizon)
            .await?;

        let missing_witnesses =
            generate_missing_witnesses(tx.id(&chain_id), &self.unresolved_signers).await?;
        tx.witnesses_mut().extend(missing_witnesses);

        Ok(tx)
    }

    pub fn with_root(mut self, root: Bytes32) -> Self {
        self.root = root;
        self
    }

    pub fn with_witness_index(mut self, witness_index: u16) -> Self {
        self.witness_index = witness_index;
        self
    }

    pub fn with_subsection_index(mut self, subsection_index: u16) -> Self {
        self.subsection_index = subsection_index;
        self
    }

    pub fn with_subsections_number(mut self, subsections_number: u16) -> Self {
        self.subsections_number = subsections_number;
        self
    }

    pub fn with_proof_set(mut self, proof_set: Vec<Bytes32>) -> Self {
        self.proof_set = proof_set;
        self
    }

    pub fn prepare_subsection_upload(
        subsection: UploadSubsection,
        tx_policies: TxPolicies,
    ) -> Self {
        let subsection_witness_index = 0;
        let outputs = vec![];
        let UploadSubsection {
            root,
            subsection,
            subsection_index,
            subsections_number,
            proof_set,
        } = subsection;
        let witnesses = vec![subsection.into()];

        Self::default()
            .with_tx_policies(tx_policies)
            .with_root(root)
            .with_witness_index(subsection_witness_index)
            .with_subsection_index(subsection_index)
            .with_subsections_number(subsections_number)
            .with_proof_set(proof_set)
            .with_outputs(outputs)
            .with_witnesses(witnesses)
    }

    fn clone_without_signers(&self) -> Self {
        Self {
            root: self.root,
            witness_index: self.witness_index,
            subsection_index: self.subsection_index,
            subsections_number: self.subsections_number,
            inputs: self.inputs.clone(),
            outputs: self.outputs.clone(),
            witnesses: self.witnesses.clone(),
            tx_policies: self.tx_policies,
            unresolved_witness_indexes: self.unresolved_witness_indexes.clone(),
            unresolved_signers: Default::default(),
            gas_price_estimation_block_horizon: self.gas_price_estimation_block_horizon,
            proof_set: vec![],
        }
    }
}

impl UpgradeTransactionBuilder {
    pub async fn build(self, provider: impl DryRunner) -> Result<UpgradeTransaction> {
        Ok(UpgradeTransaction {
            is_using_predicates: self.is_using_predicates(),
            tx: self.resolve_fuel_tx(&provider).await?,
        })
    }

    async fn resolve_fuel_tx(self, provider: impl DryRunner) -> Result<Upgrade> {
        let chain_id = provider.consensus_parameters().chain_id();
        let num_witnesses = self.num_witnesses()?;
        let policies = self.generate_fuel_policies()?;

        let mut tx = FuelTransaction::upgrade(
            self.purpose,
            policies,
            resolve_fuel_inputs(self.inputs, num_witnesses, &self.unresolved_witness_indexes)?,
            self.outputs,
            self.witnesses,
        );

        Self::set_max_fee_policy(&mut tx, provider, self.gas_price_estimation_block_horizon)
            .await?;

        let missing_witnesses =
            generate_missing_witnesses(tx.id(&chain_id), &self.unresolved_signers).await?;
        tx.witnesses_mut().extend(missing_witnesses);

        Ok(tx)
    }

    pub fn with_purpose(mut self, upgrade_purpose: UpgradePurpose) -> Self {
        self.purpose = upgrade_purpose;
        self
    }

    pub fn prepare_state_transition_upgrade(root: Bytes32, tx_policies: TxPolicies) -> Self {
        Self::default()
            .with_tx_policies(tx_policies)
            .with_purpose(UpgradePurpose::StateTransition { root })
    }

    pub fn prepare_consensus_parameters_upgrade(
        consensus_parameters: &ConsensusParameters,
        tx_policies: TxPolicies,
    ) -> Self {
        let serialized_consensus_parameters = postcard::to_allocvec(consensus_parameters)
            .expect("Impossible to fail unless there is not enough memory");
        let checksum = Hasher::hash(&serialized_consensus_parameters);
        let witness_index = 0;
        let outputs = vec![];
        let witnesses = vec![serialized_consensus_parameters.into()];

        Self::default()
            .with_tx_policies(tx_policies)
            .with_purpose(UpgradePurpose::ConsensusParameters {
                witness_index,
                checksum,
            })
            .with_outputs(outputs)
            .with_witnesses(witnesses)
    }

    fn clone_without_signers(&self) -> Self {
        Self {
            purpose: self.purpose,
            inputs: self.inputs.clone(),
            outputs: self.outputs.clone(),
            witnesses: self.witnesses.clone(),
            tx_policies: self.tx_policies,
            unresolved_witness_indexes: self.unresolved_witness_indexes.clone(),
            unresolved_signers: Default::default(),
            gas_price_estimation_block_horizon: self.gas_price_estimation_block_horizon,
        }
    }
}

/// Resolve SDK Inputs to fuel_tx Inputs. This function will calculate the right
/// data offsets for predicates and set witness indexes for signed coins.
fn resolve_fuel_inputs(
    inputs: Vec<Input>,
    num_witnesses: u16,
    unresolved_witness_indexes: &UnresolvedWitnessIndexes,
) -> Result<Vec<FuelInput>> {
    inputs
        .into_iter()
        .map(|input| match input {
            Input::ResourceSigned { resource } => {
                resolve_signed_resource(resource, num_witnesses, unresolved_witness_indexes)
            }
            Input::ResourcePredicate {
                resource,
                code,
                data,
            } => Ok(resolve_predicate_resource(resource, code, data)),
            Input::Contract {
                utxo_id,
                balance_root,
                state_root,
                tx_pointer,
                contract_id,
            } => Ok(FuelInput::contract(
                utxo_id,
                balance_root,
                state_root,
                tx_pointer,
                contract_id,
            )),
        })
        .collect()
}

fn resolve_signed_resource(
    resource: CoinType,
    num_witnesses: u16,
    unresolved_witness_indexes: &UnresolvedWitnessIndexes,
) -> Result<FuelInput> {
    match resource {
        CoinType::Coin(coin) => {
            let owner = &coin.owner;

            unresolved_witness_indexes
                .owner_to_idx_offset
                .get(owner)
                .ok_or(error_transaction!(
                    Builder,
                    "signature missing for coin with owner: `{owner:?}`"
                ))
                .map(|witness_idx_offset| {
                    create_coin_input(coin, num_witnesses + *witness_idx_offset as u16)
                })
        }
        CoinType::Message(message) => {
            let recipient = &message.recipient;

            unresolved_witness_indexes
                .owner_to_idx_offset
                .get(recipient)
                .ok_or(error_transaction!(
                    Builder,
                    "signature missing for message with recipient: `{recipient:?}`"
                ))
                .map(|witness_idx_offset| {
                    create_coin_message_input(message, num_witnesses + *witness_idx_offset as u16)
                })
        }
    }
}

fn resolve_predicate_resource(resource: CoinType, code: Vec<u8>, data: Vec<u8>) -> FuelInput {
    match resource {
        CoinType::Coin(coin) => create_coin_predicate(coin.asset_id, coin, code, data),
        CoinType::Message(message) => create_coin_message_predicate(message, code, data),
    }
}

pub fn create_coin_input(coin: Coin, witness_index: u16) -> FuelInput {
    FuelInput::coin_signed(
        coin.utxo_id,
        coin.owner.into(),
        coin.amount,
        coin.asset_id,
        TxPointer::default(),
        witness_index,
    )
}

pub fn create_coin_message_input(message: Message, witness_index: u16) -> FuelInput {
    if message.data.is_empty() {
        FuelInput::message_coin_signed(
            message.sender.into(),
            message.recipient.into(),
            message.amount,
            message.nonce,
            witness_index,
        )
    } else {
        FuelInput::message_data_signed(
            message.sender.into(),
            message.recipient.into(),
            message.amount,
            message.nonce,
            witness_index,
            message.data,
        )
    }
}

pub fn create_coin_predicate(
    asset_id: AssetId,
    coin: Coin,
    code: Vec<u8>,
    predicate_data: Vec<u8>,
) -> FuelInput {
    FuelInput::coin_predicate(
        coin.utxo_id,
        coin.owner.into(),
        coin.amount,
        asset_id,
        TxPointer::default(),
        0u64,
        code,
        predicate_data,
    )
}

pub fn create_coin_message_predicate(
    message: Message,
    code: Vec<u8>,
    predicate_data: Vec<u8>,
) -> FuelInput {
    if message.data.is_empty() {
        FuelInput::message_coin_predicate(
            message.sender.into(),
            message.recipient.into(),
            message.amount,
            message.nonce,
            0u64,
            code,
            predicate_data,
        )
    } else {
        FuelInput::message_data_predicate(
            message.sender.into(),
            message.recipient.into(),
            message.amount,
            message.nonce,
            0u64,
            message.data,
            code,
            predicate_data,
        )
    }
}

async fn generate_missing_witnesses(
    id: Bytes32,
    unresolved_signatures: &[Box<dyn Signer + Send + Sync>],
) -> Result<Vec<Witness>> {
    let mut witnesses = Vec::with_capacity(unresolved_signatures.len());
    for signer in unresolved_signatures {
        let message = CryptoMessage::from_bytes(*id);
        let signature = signer.sign(message).await?;

        witnesses.push(signature.as_ref().into());
    }

    Ok(witnesses)
}

#[cfg(test)]
mod tests {
    use std::iter::repeat_with;

    use fuel_crypto::Signature;
    use fuel_tx::{input::coin::CoinSigned, UtxoId};

    use super::*;
    use crate::types::{bech32::Bech32Address, message::MessageStatus};

    #[test]
    fn storage_slots_are_sorted_when_set() {
        let unsorted_storage_slots = [2, 1].map(given_a_storage_slot).to_vec();
        let sorted_storage_slots = [1, 2].map(given_a_storage_slot).to_vec();

        let builder =
            CreateTransactionBuilder::default().with_storage_slots(unsorted_storage_slots);

        assert_eq!(builder.storage_slots, sorted_storage_slots);
    }

    fn given_a_storage_slot(key: u8) -> StorageSlot {
        let mut bytes_32 = Bytes32::zeroed();
        bytes_32[0] = key;

        StorageSlot::new(bytes_32, Default::default())
    }

    #[test]
    fn create_message_coin_signed_if_data_is_empty() {
        assert!(matches!(
            create_coin_message_input(given_a_message(vec![]), 0),
            FuelInput::MessageCoinSigned(_)
        ));
    }

    #[test]
    fn create_message_data_signed_if_data_is_not_empty() {
        assert!(matches!(
            create_coin_message_input(given_a_message(vec![42]), 0),
            FuelInput::MessageDataSigned(_)
        ));
    }

    #[test]
    fn create_message_coin_predicate_if_data_is_empty() {
        assert!(matches!(
            create_coin_message_predicate(given_a_message(vec![]), vec![], vec![]),
            FuelInput::MessageCoinPredicate(_)
        ));
    }

    #[test]
    fn create_message_data_predicate_if_data_is_not_empty() {
        assert!(matches!(
            create_coin_message_predicate(given_a_message(vec![42]), vec![], vec![]),
            FuelInput::MessageDataPredicate(_)
        ));
    }

    fn given_a_message(data: Vec<u8>) -> Message {
        Message {
            sender: Bech32Address::default(),
            recipient: Bech32Address::default(),
            nonce: 0.into(),
            amount: 0,
            data,
            da_height: 0,
            status: MessageStatus::Unspent,
        }
    }

    fn given_inputs(num_inputs: u8) -> Vec<Input> {
        (0..num_inputs)
            .map(|i| {
                let bytes = [i; 32];
                let coin = CoinType::Coin(Coin {
                    utxo_id: UtxoId::new(bytes.into(), 0),
                    owner: Bech32Address::new("fuel", bytes),
                    ..Default::default()
                });
                Input::resource_signed(coin)
            })
            .collect()
    }

    fn given_witnesses(num_witnesses: usize) -> Vec<Witness> {
        repeat_with(Witness::default).take(num_witnesses).collect()
    }

    struct MockDryRunner {
        c_param: ConsensusParameters,
    }

    impl Default for MockDryRunner {
        fn default() -> Self {
            Self {
                c_param: ConsensusParameters::standard(),
            }
        }
    }

    #[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
    #[cfg_attr(not(target_arch = "wasm32"), async_trait)]
    impl DryRunner for MockDryRunner {
        async fn dry_run_and_get_used_gas(&self, _: FuelTransaction, _: f32) -> Result<u64> {
            Ok(0)
        }

        fn consensus_parameters(&self) -> &ConsensusParameters {
            &self.c_param
        }

        async fn estimate_gas_price(&self, _block_horizon: u32) -> Result<u64> {
            Ok(0)
        }
    }

    #[tokio::test]
    async fn create_tx_builder_witness_indexes_set_correctly() -> Result<()> {
        // given
        let num_witnesses = 2;
        let num_inputs = 3;

        let tb = CreateTransactionBuilder::default()
            .with_witnesses(given_witnesses(num_witnesses))
            .with_inputs(given_inputs(num_inputs));

        // when
        let tx = tb
            .build_without_signatures(&MockDryRunner::default())
            .await?;

        // then
        let indexes: Vec<usize> = tx
            .inputs()
            .iter()
            .filter_map(|input| match input {
                FuelInput::CoinSigned(CoinSigned { witness_index, .. }) => {
                    Some(*witness_index as usize)
                }
                _ => None,
            })
            .collect();

        let expected_indexes: Vec<_> =
            (num_witnesses..(num_witnesses + num_inputs as usize)).collect();

        assert_eq!(indexes, expected_indexes);

        Ok(())
    }

    #[tokio::test]
    async fn script_tx_builder_witness_indexes_set_correctly() -> Result<()> {
        // given
        let num_witnesses = 6;
        let num_inputs = 4;

        let tb = ScriptTransactionBuilder::default()
            .with_witnesses(given_witnesses(num_witnesses))
            .with_inputs(given_inputs(num_inputs));

        // when
        let tx = tb
            .build_without_signatures(&MockDryRunner::default())
            .await?;

        // then
        let indexes: Vec<usize> = tx
            .inputs()
            .iter()
            .filter_map(|input| match input {
                FuelInput::CoinSigned(CoinSigned { witness_index, .. }) => {
                    Some(*witness_index as usize)
                }
                _ => None,
            })
            .collect();

        let expected_indexes: Vec<_> =
            (num_witnesses..(num_witnesses + num_inputs as usize)).collect();

        assert_eq!(indexes, expected_indexes);

        Ok(())
    }

    #[derive(Clone, Debug, Default)]
    struct MockSigner {
        address: Bech32Address,
    }

    #[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
    #[cfg_attr(not(target_arch = "wasm32"), async_trait)]
    impl Signer for MockSigner {
        async fn sign(&self, _message: CryptoMessage) -> Result<Signature> {
            Ok(Default::default())
        }
        fn address(&self) -> &Bech32Address {
            &self.address
        }
    }

    #[tokio::test]
    #[should_panic(expected = "already added `Signer` with address:")]
    async fn add_signer_called_multiple_times() {
        let mut tb = ScriptTransactionBuilder::default();
        let signer = MockSigner::default();

        tb.add_signer(signer.clone()).unwrap();
        tb.add_signer(signer.clone()).unwrap();
    }
}
