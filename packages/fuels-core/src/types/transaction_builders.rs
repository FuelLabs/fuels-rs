#![cfg(feature = "std")]

use std::collections::HashMap;

use fuel_asm::{op, GTFArgs, RegId};
use fuel_crypto::{Message as CryptoMessage, SecretKey, Signature};
use fuel_tx::{
    field::{GasLimit, GasPrice, Witnesses},
    Cacheable, ConsensusParameters, Create, Input as FuelInput, Output, Script, StorageSlot,
    Transaction as FuelTransaction, TransactionFee, TxPointer, UniqueIdentifier, Witness,
};
use fuel_types::{bytes::padded_len_usize, Bytes32, MemLayout, Salt};
use fuel_vm::{checked_transaction::EstimatePredicates, gas::GasCosts};

use super::unresolved_bytes::UnresolvedBytes;
use crate::{
    constants::{BASE_ASSET_ID, WORD_SIZE},
    offsets,
    types::{
        bech32::Bech32Address,
        coin::Coin,
        coin_type::CoinType,
        errors::{error, Error, Result},
        input::Input,
        message::Message,
        transaction::{CreateTransaction, ScriptTransaction, Transaction, TxParameters},
        Address, AssetId, ContractId,
    },
};

#[derive(Debug, Clone, Default)]
struct UnresolvedSignatures {
    addr_idx_offset_map: HashMap<Bech32Address, u8>,
    secret_keys: Vec<SecretKey>,
}

pub trait TransactionBuilder: Send {
    type TxType: Transaction;

    fn build(self) -> Result<Self::TxType>;
    fn add_unresolved_signature(&mut self, owner: Bech32Address, secret_key: SecretKey);
    fn fee_checked_from_tx(&self, params: &ConsensusParameters) -> Result<Option<TransactionFee>>;
    fn with_maturity(self, maturity: u32) -> Self;
    fn with_gas_price(self, gas_price: u64) -> Self;
    fn with_gas_limit(self, gas_limit: u64) -> Self;
    fn with_tx_params(self, tx_params: TxParameters) -> Self;
    fn with_inputs(self, inputs: Vec<Input>) -> Self;
    fn with_outputs(self, outputs: Vec<Output>) -> Self;
    fn with_witnesses(self, witnesses: Vec<Witness>) -> Self;
    fn with_consensus_parameters(self, consensus_parameters: ConsensusParameters) -> Self;
    fn inputs(&self) -> &Vec<Input>;
    fn inputs_mut(&mut self) -> &mut Vec<Input>;
    fn outputs(&self) -> &Vec<Output>;
    fn outputs_mut(&mut self) -> &mut Vec<Output>;
    fn witnesses(&self) -> &Vec<Witness>;
    fn witnesses_mut(&mut self) -> &mut Vec<Witness>;
}

macro_rules! impl_tx_trait {
    ($ty: ty, $tx_ty: ident) => {
        impl TransactionBuilder for $ty {
            type TxType = $tx_ty;
            fn build(self) -> Result<$tx_ty> {
                let uses_predicates = self.is_using_predicates();
                let (base_offset, consensus_parameters) = if uses_predicates {
                    let consensus_params = self
                        .consensus_parameters
                        .ok_or(error!(
                                TransactionBuildError,
                                "predicate inputs require consensus parameters. Use `.set_consensus_parameters()`."))?;
                    (self.base_offset(&consensus_params), consensus_params)
                } else {
                    // If no ConsensusParameters have been set, we can use the default instead of
                    // erroring out since the tx doesn't use predicates
                    (0, self.consensus_parameters.unwrap_or_default())
                };

                let num_witnesses = self.num_witnesses()?;
                let mut tx =
                    self.resolve_fuel_tx(base_offset, num_witnesses, &consensus_parameters)?;

                tx.precompute(&consensus_parameters.chain_id)?;

                if uses_predicates {
                    estimate_predicates(&mut tx, &consensus_parameters)?;
                };

                Ok($tx_ty { tx })
            }

            fn add_unresolved_signature(&mut self, owner: Bech32Address, secret_key: SecretKey) {
                let index_offset = self.unresolved_signatures.secret_keys.len() as u8;
                self.unresolved_signatures.secret_keys.push(secret_key);
                self.unresolved_signatures.addr_idx_offset_map.insert(owner, index_offset);
            }

            fn fee_checked_from_tx(&self, params: &ConsensusParameters) -> Result<Option<TransactionFee>>{
                let tx = self.clone().build()?.tx;
                Ok(TransactionFee::checked_from_tx(params, &tx))
            }

            fn with_maturity(mut self, maturity: u32) -> Self {
                self.maturity = maturity.into();
                self
            }

            fn with_gas_price(mut self, gas_price: u64) -> Self {
                self.gas_price = gas_price;
                self
            }

            fn with_gas_limit(mut self, gas_limit: u64) -> Self {
                self.gas_limit = gas_limit;
                self
            }

            fn with_tx_params(self, tx_params: TxParameters) -> Self {
                self.with_gas_limit(tx_params.gas_limit())
                    .with_gas_price(tx_params.gas_price())
                    .with_maturity(tx_params.maturity().into())
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

            fn with_consensus_parameters(
                mut self,
                consensus_parameters: ConsensusParameters,
            ) -> Self {
                self.consensus_parameters = Some(consensus_parameters);
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
        }

        impl $ty {
            fn is_using_predicates(&self) -> bool {
                self.inputs()
                    .iter()
                    .any(|input| matches!(input, Input::ResourcePredicate { .. }))
            }

            fn num_witnesses(&self) -> Result<u8> {
                let num_witnesses = self
                    .witnesses()
                    .len();

                if num_witnesses + self.unresolved_signatures.secret_keys.len() > 256 {
                    return Err(error!(InvalidData, "tx can not have more than 256 witnesses"));
                }

                Ok(num_witnesses as u8)
            }
        }
    };
}

#[derive(Debug, Clone, Default)]
pub struct ScriptTransactionBuilder {
    pub gas_price: u64,
    pub gas_limit: u64,
    pub maturity: u32,
    pub script: Vec<u8>,
    pub script_data: Vec<u8>,
    pub inputs: Vec<Input>,
    pub outputs: Vec<Output>,
    pub witnesses: Vec<Witness>,
    pub(crate) consensus_parameters: Option<ConsensusParameters>,
    unresolved_signatures: UnresolvedSignatures,
}

#[derive(Debug, Clone, Default)]
pub struct CreateTransactionBuilder {
    pub gas_price: u64,
    pub gas_limit: u64,
    pub maturity: u32,
    pub bytecode_length: u64,
    pub bytecode_witness_index: u8,
    pub storage_slots: Vec<StorageSlot>,
    pub inputs: Vec<Input>,
    pub outputs: Vec<Output>,
    pub witnesses: Vec<Witness>,
    pub salt: Salt,
    pub(crate) consensus_parameters: Option<ConsensusParameters>,
    unresolved_signatures: UnresolvedSignatures,
}

impl_tx_trait!(ScriptTransactionBuilder, ScriptTransaction);
impl_tx_trait!(CreateTransactionBuilder, CreateTransaction);

impl ScriptTransactionBuilder {
    fn resolve_fuel_tx(
        self,
        base_offset: usize,
        num_witnesses: u8,
        consensus_parameters: &ConsensusParameters,
    ) -> Result<Script> {
        let mut tx = FuelTransaction::script(
            self.gas_price,
            self.gas_limit,
            self.maturity.into(),
            self.script,
            self.script_data,
            resolve_fuel_inputs(
                self.inputs,
                base_offset,
                num_witnesses,
                &self.unresolved_signatures,
            )?,
            self.outputs,
            self.witnesses,
        );

        let missing_witnesses = generate_missing_witnesses(
            tx.id(&consensus_parameters.chain_id),
            &self.unresolved_signatures,
        );
        tx.witnesses_mut().extend(missing_witnesses);

        Ok(tx)
    }

    fn base_offset(&self, consensus_parameters: &ConsensusParameters) -> usize {
        offsets::base_offset_script(consensus_parameters)
            + padded_len_usize(self.script_data.len())
            + padded_len_usize(self.script.len())
    }

    pub fn with_script(mut self, script: Vec<u8>) -> Self {
        self.script = script;
        self
    }

    pub fn with_script_data(mut self, script_data: Vec<u8>) -> Self {
        self.script_data = script_data;
        self
    }

    pub fn prepare_transfer(
        inputs: Vec<Input>,
        outputs: Vec<Output>,
        params: TxParameters,
    ) -> Self {
        ScriptTransactionBuilder::default()
            .with_inputs(inputs)
            .with_outputs(outputs)
            .with_tx_params(params)
    }

    /// Craft a transaction used to transfer funds to a contract.
    pub fn prepare_contract_transfer(
        to: ContractId,
        amount: u64,
        asset_id: AssetId,
        inputs: Vec<Input>,
        outputs: Vec<Output>,
        params: TxParameters,
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
            .with_tx_params(params)
            .with_script(script)
            .with_script_data(script_data)
            .with_inputs(inputs)
            .with_outputs(outputs)
    }

    /// Craft a transaction used to transfer funds to the base chain.
    pub fn prepare_message_to_output(
        to: Address,
        amount: u64,
        inputs: Vec<Input>,
        params: TxParameters,
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

        let outputs = vec![Output::change(to, 0, BASE_ASSET_ID)];

        ScriptTransactionBuilder::default()
            .with_tx_params(params)
            .with_script(script)
            .with_script_data(script_data)
            .with_inputs(inputs)
            .with_outputs(outputs)
    }
}

impl CreateTransactionBuilder {
    fn resolve_fuel_tx(
        self,
        base_offset: usize,
        num_witnesses: u8,
        consensus_parameters: &ConsensusParameters,
    ) -> Result<Create> {
        let num_of_storage_slots = self.storage_slots.len();
        let mut tx = FuelTransaction::create(
            self.gas_price,
            self.gas_limit,
            self.maturity.into(),
            self.bytecode_witness_index,
            self.salt,
            self.storage_slots,
            resolve_fuel_inputs(
                self.inputs,
                base_offset + num_of_storage_slots * StorageSlot::LEN,
                num_witnesses,
                &self.unresolved_signatures,
            )?,
            self.outputs,
            self.witnesses,
        );

        let missing_witnesses = generate_missing_witnesses(
            tx.id(&consensus_parameters.chain_id),
            &self.unresolved_signatures,
        );

        tx.witnesses_mut().extend(missing_witnesses);

        Ok(tx)
    }

    fn base_offset(&self, consensus_parameters: &ConsensusParameters) -> usize {
        offsets::base_offset_create(consensus_parameters)
    }

    pub fn with_bytecode_length(mut self, bytecode_length: u64) -> Self {
        self.bytecode_length = bytecode_length;
        self
    }

    pub fn with_bytecode_witness_index(mut self, bytecode_witness_index: u8) -> Self {
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
        params: TxParameters,
    ) -> Self {
        let bytecode_witness_index = 0;
        let outputs = vec![Output::contract_created(contract_id, state_root)];
        let witnesses = vec![binary.into()];

        CreateTransactionBuilder::default()
            .with_tx_params(params)
            .with_bytecode_witness_index(bytecode_witness_index)
            .with_salt(salt)
            .with_storage_slots(storage_slots)
            .with_outputs(outputs)
            .with_witnesses(witnesses)
    }
}

/// Resolve SDK Inputs to fuel_tx Inputs. This function will calculate the right
/// data offsets for predicates and set witness indexes for signed coins.
fn resolve_fuel_inputs(
    inputs: Vec<Input>,
    mut data_offset: usize,
    num_witnesses: u8,
    unresolved_signatures: &UnresolvedSignatures,
) -> Result<Vec<FuelInput>> {
    inputs
        .into_iter()
        .map(|input| match input {
            Input::ResourceSigned { resource } => resolve_signed_resource(
                resource,
                &mut data_offset,
                num_witnesses,
                unresolved_signatures,
            ),
            Input::ResourcePredicate {
                resource,
                code,
                data,
            } => resolve_predicate_resource(resource, code, data, &mut data_offset),
            Input::Contract {
                utxo_id,
                balance_root,
                state_root,
                tx_pointer,
                contract_id,
            } => {
                data_offset += offsets::contract_input_offset();
                Ok(FuelInput::contract(
                    utxo_id,
                    balance_root,
                    state_root,
                    tx_pointer,
                    contract_id,
                ))
            }
        })
        .collect()
}

fn resolve_signed_resource(
    resource: CoinType,
    data_offset: &mut usize,
    num_witnesses: u8,
    unresolved_signatures: &UnresolvedSignatures,
) -> Result<FuelInput> {
    match resource {
        CoinType::Coin(coin) => {
            *data_offset += offsets::coin_signed_data_offset();
            let owner = &coin.owner;

            unresolved_signatures
                .addr_idx_offset_map
                .get(owner)
                .ok_or(error!(
                    InvalidData,
                    "signature missing for coin with owner: `{owner:?}`"
                ))
                .map(|witness_idx_offset| {
                    create_coin_input(coin, num_witnesses + *witness_idx_offset)
                })
        }
        CoinType::Message(message) => {
            *data_offset += offsets::message_signed_data_offset(message.data.len());
            let recipient = &message.recipient;

            unresolved_signatures
                .addr_idx_offset_map
                .get(recipient)
                .ok_or(error!(
                    InvalidData,
                    "signature missing for message with recipient: `{recipient:?}`"
                ))
                .map(|witness_idx_offset| {
                    create_coin_message_input(message, num_witnesses + *witness_idx_offset)
                })
        }
    }
}

fn resolve_predicate_resource(
    resource: CoinType,
    code: Vec<u8>,
    data: UnresolvedBytes,
    data_offset: &mut usize,
) -> Result<FuelInput> {
    match resource {
        CoinType::Coin(coin) => {
            *data_offset += offsets::coin_predicate_data_offset(code.len());

            let data = data.resolve(*data_offset as u64);
            *data_offset += data.len();

            let asset_id = coin.asset_id;
            Ok(create_coin_predicate(coin, asset_id, code, data))
        }
        CoinType::Message(message) => {
            *data_offset += offsets::message_predicate_data_offset(message.data.len(), code.len());

            let data = data.resolve(*data_offset as u64);
            *data_offset += data.len();

            Ok(create_coin_message_predicate(message, code, data))
        }
    }
}

pub fn create_coin_input(coin: Coin, witness_index: u8) -> FuelInput {
    FuelInput::coin_signed(
        coin.utxo_id,
        coin.owner.into(),
        coin.amount,
        coin.asset_id,
        TxPointer::default(),
        witness_index,
        0u32.into(),
    )
}

pub fn create_coin_message_input(message: Message, witness_index: u8) -> FuelInput {
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
    coin: Coin,
    asset_id: AssetId,
    code: Vec<u8>,
    predicate_data: Vec<u8>,
) -> FuelInput {
    FuelInput::coin_predicate(
        coin.utxo_id,
        coin.owner.into(),
        coin.amount,
        asset_id,
        TxPointer::default(),
        0u32.into(),
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

fn generate_missing_witnesses(
    id: Bytes32,
    unresolved_signatures: &UnresolvedSignatures,
) -> Vec<Witness> {
    unresolved_signatures
        .secret_keys
        .iter()
        .map(|secret_key| {
            let message = CryptoMessage::from_bytes(*id);
            let signature = Signature::sign(secret_key, &message);

            Witness::from(signature.as_ref())
        })
        .collect()
}

fn estimate_predicates<T>(tx: &mut T, consensus_parameters: &ConsensusParameters) -> Result<()>
where
    T: GasLimit + GasPrice + EstimatePredicates,
{
    let gas_price = *tx.gas_price();
    let gas_limit = *tx.gas_limit();
    *tx.gas_price_mut() = 0;
    *tx.gas_limit_mut() = consensus_parameters.max_gas_per_tx;

    // TODO: Fetch `GasCosts` from the `fuel-core`:
    //  https://github.com/FuelLabs/fuel-core/issues/1221
    tx.estimate_predicates(consensus_parameters, &GasCosts::default())?;
    *tx.gas_price_mut() = gas_price;
    *tx.gas_limit_mut() = gas_limit;

    Ok(())
}

#[cfg(test)]
mod tests {
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
}
