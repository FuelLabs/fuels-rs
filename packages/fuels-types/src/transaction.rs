use std::fmt::Debug;

use fuel_asm::{op, GTFArgs, RegId};
use fuel_tx::{field::{
    GasLimit, GasPrice, Inputs, Maturity, Outputs, Script as ScriptField, ScriptData, Witnesses,
}, Address, AssetId, Bytes32, Chargeable, ConsensusParameters, ContractId, Create, FormatValidityChecks, Input as FuelInputs, Output, Script, Transaction as FuelTransaction, TransactionFee, UniqueIdentifier, Witness, StorageSlot};
use fuel_types::Salt;

use crate::{
    constants::{BASE_ASSET_ID, WORD_SIZE},
    errors::Error,
    parameters::TxParameters,
};
use crate::input::Input;


pub trait Transaction: Into<FuelTransaction> {
    fn fee_checked_from_tx(&self, params: &ConsensusParameters) -> Option<TransactionFee>;

    fn check_without_signatures(
        &self,
        block_height: u64,
        parameters: &ConsensusParameters,
    ) -> Result<(), Error>;

    fn id(&self) -> Bytes32;

    fn maturity(&self) -> u64;

    fn with_maturity(self, maturity: u64) -> Self;

    fn gas_price(&self) -> u64;

    fn with_gas_price(self, gas_price: u64) -> Self;

    fn gas_limit(&self) -> u64;

    fn with_gas_limit(self, gas_price: u64) -> Self;

    fn with_tx_params(self, tx_params: TxParameters) -> Self;

    fn metered_bytes_size(&self) -> usize;

    fn inputs(&self) -> &Vec<FuelInputs>;

    fn inputs_mut(&mut self) -> &mut Vec<FuelInputs>;

    fn with_inputs(self, inputs: Vec<FuelInputs>) -> Self;

    fn outputs(&self) -> &Vec<Output>;

    fn outputs_mut(&mut self) -> &mut Vec<Output>;

    fn with_outputs(self, output: Vec<Output>) -> Self;

    fn witnesses(&self) -> &Vec<Witness>;

    fn witnesses_mut(&mut self) -> &mut Vec<Witness>;

    fn with_witnesses(self, witnesses: Vec<Witness>) -> Self;

    fn tx_offset(&self) -> usize;
}

#[derive(Debug, Clone)]
pub struct ScriptTransaction {
    pub(crate) gas_price: u64,
    pub(crate) gas_limit: u64,
    pub(crate) maturity: u64,
    pub(crate) script: Vec<u8>,
    pub(crate) script_data: Vec<u8>,
    pub(crate) inputs: Vec<Input>,
    pub(crate) outputs: Vec<Output>,
    pub(crate) witnesses: Vec<Witness>,
    pub tx_offset: usize,
}

#[derive(Debug, Clone)]
pub struct CreateTransaction {
    pub(crate) gas_price: u64,
    pub(crate) gas_limit: u64,
    pub(crate) maturity: u64,
    pub(crate) bytecode_length: u64,
    pub(crate) bytecode_witness_index: u8,
    pub(crate) storage_slots: Vec<StorageSlot>,
    pub(crate) inputs: Vec<Input>,
    pub(crate) outputs: Vec<Output>,
    pub(crate) witnesses: Vec<Witness>,
    pub(crate) salt: Salt,
    pub tx_offset: usize,
}



macro_rules! impl_tx_trait {
    ($ty: ident) => {
        impl Transaction for $ty {
            fn fee_checked_from_tx(&self, params: &ConsensusParameters) -> Option<TransactionFee> {
                TransactionFee::checked_from_tx(params, &self.tx)
            }

            fn check_without_signatures(
                &self,
                block_height: u64,
                parameters: &ConsensusParameters,
            ) -> Result<(), Error> {
                Ok(self.tx.check_without_signatures(block_height, parameters)?)
            }

            fn id(&self) -> Bytes32 {
                self.tx.id()
            }

            fn tx_offset(&self) -> usize {
                self.tx_offset
            }

            fn maturity(&self) -> u64 {
                *self.tx.maturity()
            }

            fn with_maturity(mut self, maturity: u64) -> Self {
                *self.tx.maturity_mut() = maturity;
                self
            }

            fn gas_price(&self) -> u64 {
                *self.tx.gas_price()
            }

            fn with_gas_price(mut self, gas_price: u64) -> Self {
                *self.tx.gas_price_mut() = gas_price;
                self
            }

            fn gas_limit(&self) -> u64 {
                *self.tx.gas_limit()
            }

            fn with_gas_limit(mut self, gas_limit: u64) -> Self {
                *self.tx.gas_limit_mut() = gas_limit;
                self
            }

            fn with_tx_params(self, tx_params: TxParameters) -> Self {
                self.with_gas_limit(tx_params.gas_limit)
                    .with_gas_price(tx_params.gas_price)
                    .with_maturity(tx_params.maturity)
            }

            fn metered_bytes_size(&self) -> usize {
                self.tx.metered_bytes_size()
            }

            fn inputs(&self) -> &Vec<Input> {
                self.tx.inputs()
            }

            fn inputs_mut(&mut self) -> &mut Vec<Input> {
                self.tx.inputs_mut()
            }

            fn with_inputs(mut self, inputs: Vec<Input>) -> Self {
                *self.tx.inputs_mut() = inputs;
                self
            }

            fn outputs(&self) -> &Vec<Output> {
                self.tx.outputs()
            }

            fn outputs_mut(&mut self) -> &mut Vec<Output> {
                self.tx.outputs_mut()
            }

            fn with_outputs(mut self, outputs: Vec<Output>) -> Self {
                *self.tx.outputs_mut() = outputs;
                self
            }

            fn witnesses(&self) -> &Vec<Witness> {
                self.tx.witnesses()
            }

            fn witnesses_mut(&mut self) -> &mut Vec<Witness> {
                self.tx.witnesses_mut()
            }

            fn with_witnesses(mut self, witnesses: Vec<Witness>) -> Self {
                *self.tx.witnesses_mut() = witnesses;
                self
            }
        }
    };
}

impl_tx_wrapper!(ScriptTransaction);
impl_tx_wrapper!(CreateTransaction);

impl ScriptTransaction {
    pub fn script(&self) -> &Vec<u8> {
        self.tx.script()
    }

    pub fn with_script(mut self, script: Vec<u8>) -> Self {
        *self.tx.script_mut() = script;
        self
    }

    pub fn script_data(&self) -> &Vec<u8> {
        self.tx.script_data()
    }

    pub fn with_script_data(mut self, script_data: Vec<u8>) -> Self {
        *self.tx.script_data_mut() = script_data;
        self
    }

    pub fn new(
        inputs: Vec<FuelInputs>,
        outputs: Vec<Output>,
        params: TxParameters,
    ) -> ScriptTransaction {
        FuelTransaction::script(
            params.gas_price,
            params.gas_limit,
            params.maturity,
            vec![],
            vec![],
            inputs,
            outputs,
            vec![],
        )
            .into()
    }

    /// Craft a transaction used to transfer funds to a contract.
    pub fn build_contract_transfer_tx(
        to: ContractId,
        amount: u64,
        asset_id: AssetId,
        inputs: Vec<FuelInputs>,
        outputs: Vec<Output>,
        params: TxParameters,
    ) -> ScriptTransaction {
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

        FuelTransaction::script(
            params.gas_price,
            params.gas_limit,
            params.maturity,
            script,
            script_data,
            inputs.to_vec(),
            outputs.to_vec(),
            vec![],
        )
            .into()
    }

    /// Craft a transaction used to transfer funds to the base chain.
    pub fn build_message_to_output_tx(
        to: Address,
        amount: u64,
        inputs: Vec<FuelInputs>,
        params: TxParameters,
    ) -> ScriptTransaction {
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

        FuelTransaction::script(
            params.gas_price,
            params.gas_limit,
            params.maturity,
            script,
            script_data,
            inputs,
            outputs,
            vec![],
        )
            .into()
    }
}

