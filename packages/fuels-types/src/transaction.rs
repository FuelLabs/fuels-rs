use std::fmt::Debug;

use fuel_asm::{op, GTFArgs, RegId};
use fuel_tx::{
    field::{
        GasLimit, GasPrice, Inputs, Maturity, Outputs, Script as ScriptField, ScriptData, Witnesses,
    },
    Address, AssetId, Bytes32, Chargeable, ConsensusParameters, ContractId, Create,
    FormatValidityChecks, Input, Output, Salt, Script, StorageSlot, Transaction as FuelTransaction,
    TransactionFee, UniqueIdentifier, Witness,
};

use crate::{
    constants::{BASE_ASSET_ID, DEFAULT_GAS_LIMIT, DEFAULT_GAS_PRICE, DEFAULT_MATURITY, WORD_SIZE},
    errors::Error,
};

#[derive(Debug, Copy, Clone)]
pub struct TxParameters {
    gas_price: u64,
    gas_limit: u64,
    maturity: u64,
}

macro_rules! impl_setter_getter {
    ($setter_name: ident, $field: ident) => {
        pub fn $setter_name(mut self, $field: u64) -> Self {
            self.$field = $field;
            self
        }

        pub fn $field(&self) -> u64 {
            self.$field
        }
    };
}

impl TxParameters {
    pub fn new(gas_price: u64, gas_limit: u64, maturity: u64) -> Self {
        Self {
            gas_price,
            gas_limit,
            maturity,
        }
    }

    impl_setter_getter!(set_gas_price, gas_price);
    impl_setter_getter!(set_gas_limit, gas_limit);
    impl_setter_getter!(set_maturity, maturity);
}

impl Default for TxParameters {
    fn default() -> Self {
        Self {
            gas_price: DEFAULT_GAS_PRICE,
            gas_limit: DEFAULT_GAS_LIMIT,
            // By default, transaction is immediately valid
            maturity: DEFAULT_MATURITY,
        }
    }
}

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
    fn inputs(&self) -> &Vec<Input>;
    fn inputs_mut(&mut self) -> &mut Vec<Input>;
    fn with_inputs(self, inputs: Vec<Input>) -> Self;
    fn outputs(&self) -> &Vec<Output>;
    fn outputs_mut(&mut self) -> &mut Vec<Output>;
    fn with_outputs(self, output: Vec<Output>) -> Self;
    fn witnesses(&self) -> &Vec<Witness>;
    fn witnesses_mut(&mut self) -> &mut Vec<Witness>;
    fn with_witnesses(self, witnesses: Vec<Witness>) -> Self;
}

macro_rules! impl_tx_wrapper {
    ($wrapper: ident, $wrapped: ident) => {
        #[derive(Debug, Clone)]
        pub struct $wrapper {
            pub tx: $wrapped,
        }

        impl From<$wrapped> for $wrapper {
            fn from(tx: $wrapped) -> Self {
                $wrapper { tx }
            }
        }

        impl Default for $wrapper {
            fn default() -> Self {
                $wrapped::default().into()
            }
        }

        impl From<$wrapper> for $wrapped {
            fn from(tx: $wrapper) -> Self {
                tx.tx
            }
        }

        impl From<$wrapper> for FuelTransaction {
            fn from(tx: $wrapper) -> Self {
                tx.tx.into()
            }
        }

        impl Transaction for $wrapper {
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

impl_tx_wrapper!(ScriptTransaction, Script);
impl_tx_wrapper!(CreateTransaction, Create);

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
        inputs: Vec<Input>,
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
        inputs: Vec<Input>,
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
        inputs: Vec<Input>,
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
        let script = vec![
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

impl CreateTransaction {
    pub fn build_contract_deployment_tx(
        bytecode_witness_index: u8,
        outputs: Vec<Output>,
        witnesses: Vec<Witness>,
        salt: Salt,
        storage_slots: Vec<StorageSlot>,
        params: TxParameters,
    ) -> Self {
        FuelTransaction::create(
            params.gas_price,
            params.gas_limit,
            params.maturity,
            bytecode_witness_index,
            salt,
            storage_slots,
            vec![],
            outputs,
            witnesses,
        )
        .into()
    }
}
