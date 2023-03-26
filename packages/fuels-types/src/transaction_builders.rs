#![cfg(feature = "std")]

use fuel_asm::{op, GTFArgs, RegId};
use fuel_tx::{
    ConsensusParameters, FormatValidityChecks, Input as FuelInput, Output, StorageSlot,
    Transaction as FuelTransaction, TransactionFee, TxPointer, Witness,
};
use fuel_types::{bytes::padded_len_usize, Address, AssetId, Bytes32, ContractId, Salt};

use crate::{
    coin::Coin,
    constants::{BASE_ASSET_ID, WORD_SIZE},
    errors::{Error, Result},
    input::Input,
    message::Message,
    offsets,
    resource::Resource,
    transaction::{CreateTransaction, ScriptTransaction, Transaction, TxParameters},
};

pub trait TransactionBuilder: Send {
    type TxType: Transaction;

    fn build(self) -> Result<Self::TxType>;

    fn fee_checked_from_tx(&self, params: &ConsensusParameters) -> Option<TransactionFee>;

    fn check_without_signatures(
        &self,
        block_height: u64,
        parameters: &ConsensusParameters,
    ) -> Result<()>;

    fn set_maturity(self, maturity: u64) -> Self;
    fn set_gas_price(self, gas_price: u64) -> Self;
    fn set_gas_limit(self, gas_limit: u64) -> Self;
    fn set_tx_params(self, tx_params: TxParameters) -> Self;
    fn set_inputs(self, inputs: Vec<Input>) -> Self;
    fn set_outputs(self, outputs: Vec<Output>) -> Self;
    fn set_witnesses(self, witnesses: Vec<Witness>) -> Self;
    fn set_consensus_parameters(self, consensus_parameters: ConsensusParameters) -> Self;
    fn inputs(&self) -> &Vec<Input>;
    fn inputs_mut(&mut self) -> &mut Vec<Input>;
    fn outputs(&self) -> &Vec<Output>;
    fn outputs_mut(&mut self) -> &mut Vec<Output>;
    fn witnesses(&self) -> &Vec<Witness>;
    fn witnesses_mut(&mut self) -> &mut Vec<Witness>;
}

macro_rules! impl_tx_trait {
    ($ty: ty, $tx_ty: ty) => {
        impl TransactionBuilder for $ty {
            type TxType = $tx_ty;
            fn build(self) -> Result<$tx_ty> {
                let base_offset = if self.is_using_predicates() {
                    let params = self
                        .consensus_parameters
                        .ok_or(Error::TransactionBuildError)?;
                    self.base_offset(&params)
                } else {
                    0
                };

                Ok(self.convert_to_fuel_tx(base_offset))
            }

            fn fee_checked_from_tx(&self, params: &ConsensusParameters) -> Option<TransactionFee> {
                let tx = &self.clone().build().expect("Error in build").tx;
                TransactionFee::checked_from_tx(params, tx)
            }

            fn check_without_signatures(
                &self,
                block_height: u64,
                parameters: &ConsensusParameters,
            ) -> Result<()> {
                Ok(self
                    .clone()
                    .build()
                    .expect("Error in build")
                    .tx
                    .check_without_signatures(block_height, parameters)?)
            }

            fn set_maturity(mut self, maturity: u64) -> Self {
                self.maturity = maturity;
                self
            }

            fn set_gas_price(mut self, gas_price: u64) -> Self {
                self.gas_price = gas_price;
                self
            }

            fn set_gas_limit(mut self, gas_limit: u64) -> Self {
                self.gas_limit = gas_limit;
                self
            }

            fn set_tx_params(self, tx_params: TxParameters) -> Self {
                self.set_gas_limit(tx_params.gas_limit())
                    .set_gas_price(tx_params.gas_price())
                    .set_maturity(tx_params.maturity())
            }

            fn set_inputs(mut self, inputs: Vec<Input>) -> Self {
                self.inputs = inputs;
                self
            }

            fn set_outputs(mut self, outputs: Vec<Output>) -> Self {
                self.outputs = outputs;
                self
            }

            fn set_witnesses(mut self, witnesses: Vec<Witness>) -> Self {
                self.witnesses = witnesses;
                self
            }

            fn set_consensus_parameters(
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
        }
    };
}

#[derive(Debug, Clone, Default)]
pub struct ScriptTransactionBuilder {
    pub gas_price: u64,
    pub gas_limit: u64,
    pub maturity: u64,
    pub script: Vec<u8>,
    pub script_data: Vec<u8>,
    pub inputs: Vec<Input>,
    pub outputs: Vec<Output>,
    pub witnesses: Vec<Witness>,
    pub(crate) consensus_parameters: Option<ConsensusParameters>,
}

#[derive(Debug, Clone, Default)]
pub struct CreateTransactionBuilder {
    pub gas_price: u64,
    pub gas_limit: u64,
    pub maturity: u64,
    pub bytecode_length: u64,
    pub bytecode_witness_index: u8,
    pub storage_slots: Vec<StorageSlot>,
    pub inputs: Vec<Input>,
    pub outputs: Vec<Output>,
    pub witnesses: Vec<Witness>,
    pub salt: Salt,
    pub(crate) consensus_parameters: Option<ConsensusParameters>,
}

impl_tx_trait!(ScriptTransactionBuilder, ScriptTransaction);
impl_tx_trait!(CreateTransactionBuilder, CreateTransaction);

impl ScriptTransactionBuilder {
    fn convert_to_fuel_tx(self, base_offset: usize) -> ScriptTransaction {
        FuelTransaction::script(
            self.gas_price,
            self.gas_limit,
            self.maturity,
            self.script,
            self.script_data,
            convert_to_fuel_inputs(&self.inputs, base_offset),
            self.outputs,
            self.witnesses,
        )
        .into()
    }

    fn base_offset(&self, consensus_parameters: &ConsensusParameters) -> usize {
        offsets::base_offset_script(consensus_parameters)
            + padded_len_usize(self.script_data.len())
            + padded_len_usize(self.script.len())
    }

    pub fn set_script(mut self, script: Vec<u8>) -> Self {
        self.script = script;
        self
    }

    pub fn set_script_data(mut self, script_data: Vec<u8>) -> Self {
        self.script_data = script_data;
        self
    }

    pub fn prepare_transfer(
        inputs: Vec<Input>,
        outputs: Vec<Output>,
        params: TxParameters,
    ) -> Self {
        ScriptTransactionBuilder::default()
            .set_inputs(inputs)
            .set_outputs(outputs)
            .set_tx_params(params)
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
            .set_tx_params(params)
            .set_script(script)
            .set_script_data(script_data)
            .set_inputs(inputs)
            .set_outputs(outputs)
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

        let outputs = vec![
            // when signing a transaction, recipient and amount are set to zero
            Output::message(Address::zeroed(), 0),
            Output::change(to, 0, BASE_ASSET_ID),
        ];

        ScriptTransactionBuilder::default()
            .set_tx_params(params)
            .set_script(script)
            .set_script_data(script_data)
            .set_inputs(inputs)
            .set_outputs(outputs)
    }
}

impl CreateTransactionBuilder {
    fn convert_to_fuel_tx(self, base_offset: usize) -> CreateTransaction {
        FuelTransaction::create(
            self.gas_price,
            self.gas_limit,
            self.maturity,
            self.bytecode_witness_index,
            self.salt,
            self.storage_slots,
            convert_to_fuel_inputs(&self.inputs, base_offset), //placeholder offset
            self.outputs,
            self.witnesses,
        )
        .into()
    }

    fn base_offset(&self, consensus_parameters: &ConsensusParameters) -> usize {
        offsets::base_offset_create(consensus_parameters)
    }

    pub fn set_bytecode_length(mut self, bytecode_length: u64) -> Self {
        self.bytecode_length = bytecode_length;
        self
    }

    pub fn set_bytecode_witness_index(mut self, bytecode_witness_index: u8) -> Self {
        self.bytecode_witness_index = bytecode_witness_index;
        self
    }

    pub fn set_storage_slots(mut self, mut storage_slots: Vec<StorageSlot>) -> Self {
        // Storage slots have to be sorted otherwise we'd get a `TransactionCreateStorageSlotOrder`
        // error.
        storage_slots.sort();
        self.storage_slots = storage_slots;
        self
    }

    pub fn set_salt(mut self, salt: Salt) -> Self {
        self.salt = salt;
        self
    }
}

fn convert_to_fuel_inputs(inputs: &[Input], offset: usize) -> Vec<FuelInput> {
    let mut new_offset = offset;

    inputs
        .iter()
        .map(|input| match input {
            Input::ResourcePredicate {
                resource: Resource::Coin(coin),
                code,
                data,
            } => {
                new_offset += offsets::coin_predicate_data_offset(code.len());

                let data = data.clone().resolve(new_offset as u64);
                new_offset += data.len();

                create_coin_predicate(coin.clone(), coin.asset_id, code.clone(), data)
            }
            Input::ResourcePredicate {
                resource: Resource::Message(message),
                code,
                data,
            } => {
                new_offset +=
                    offsets::message_predicate_data_offset(message.data.len(), code.len());

                let data = data.clone().resolve(new_offset as u64);
                new_offset += data.len();

                create_message_predicate(message.clone(), code.clone(), data)
            }
            Input::ResourceSigned {
                resource,
                witness_index,
            } => match resource {
                Resource::Coin(coin) => {
                    new_offset += offsets::coin_signed_data_offset();
                    create_coin_input(coin.clone(), *witness_index)
                }
                Resource::Message(message) => {
                    new_offset += offsets::message_signed_data_offset(message.data.len());
                    create_message_input(message.clone(), *witness_index)
                }
            },
            Input::Contract {
                utxo_id,
                balance_root,
                state_root,
                tx_pointer,
                contract_id,
            } => {
                new_offset += offsets::contract_input_offset();
                FuelInput::contract(
                    *utxo_id,
                    *balance_root,
                    *state_root,
                    *tx_pointer,
                    *contract_id,
                )
            }
        })
        .collect::<Vec<FuelInput>>()
}

pub fn create_coin_input(coin: Coin, witness_index: u8) -> FuelInput {
    FuelInput::coin_signed(
        coin.utxo_id,
        coin.owner.into(),
        coin.amount,
        coin.asset_id,
        TxPointer::default(),
        witness_index,
        0,
    )
}

pub fn create_message_input(message: Message, witness_index: u8) -> FuelInput {
    FuelInput::message_signed(
        message.message_id(),
        message.sender.into(),
        message.recipient.into(),
        message.amount,
        message.nonce,
        witness_index,
        message.data,
    )
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
        0,
        code,
        predicate_data,
    )
}

pub fn create_message_predicate(
    message: Message,
    code: Vec<u8>,
    predicate_data: Vec<u8>,
) -> FuelInput {
    FuelInput::message_predicate(
        message.message_id(),
        message.sender.into(),
        message.recipient.into(),
        message.amount,
        message.nonce,
        message.data,
        code,
        predicate_data,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn storage_slots_are_sorted_when_set() {
        let unsorted_storage_slots = [2, 1].map(given_a_storage_slot).to_vec();
        let sorted_storage_slots = [1, 2].map(given_a_storage_slot).to_vec();

        let builder = CreateTransactionBuilder::default().set_storage_slots(unsorted_storage_slots);

        assert_eq!(builder.storage_slots, sorted_storage_slots);
    }

    fn given_a_storage_slot(key: u8) -> StorageSlot {
        let mut bytes_32 = Bytes32::zeroed();
        bytes_32[0] = key;

        StorageSlot::new(bytes_32, Default::default())
    }
}
