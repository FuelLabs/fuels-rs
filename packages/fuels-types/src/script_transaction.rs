use std::fmt::Debug;

use fuel_tx::field::{
    GasLimit, GasPrice, Inputs, Maturity, Outputs, Script as ScriptField, ScriptData, Witnesses,
};
use fuel_tx::{
    Address, AssetId, Bytes32, Chargeable, ConsensusParameters, ContractId, Create,
    FormatValidityChecks, Input, Output, Script, Transaction as FuelTransaction, TransactionFee,
    UniqueIdentifier, Witness,
};
use fuel_vm::fuel_asm::{op, RegId};
use fuel_vm::prelude::GTFArgs;

use crate::constants::{BASE_ASSET_ID, WORD_SIZE};
use crate::errors::Error;
use crate::parameters::TxParameters;

pub trait Transaction: Into<FuelTransaction> {
    fn fee_checked_from_tx(&self, params: &ConsensusParameters) -> Option<TransactionFee>;

    fn check_without_signatures(
        &self,
        block_height: u64,
        parameters: &ConsensusParameters,
    ) -> Result<(), Error>;

    fn id(&self) -> Bytes32;

    fn maturity(&self) -> u64;

    fn maturity_mut(&mut self) -> &mut u64;

    fn gas_price_mut(&mut self) -> &mut u64;

    fn gas_price(&self) -> u64;

    fn gas_limit(&self) -> u64;

    fn gas_limit_mut(&mut self) -> &mut u64;

    fn metered_bytes_size(&self) -> usize;

    fn inputs(&self) -> &Vec<Input>;

    fn inputs_mut(&mut self) -> &mut Vec<Input>;

    fn outputs(&self) -> &Vec<Output>;

    fn outputs_mut(&mut self) -> &mut Vec<Output>;

    fn witnesses(&self) -> &Vec<Witness>;

    fn witnesses_mut(&mut self) -> &mut Vec<Witness>;
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

            fn maturity_mut(&mut self) -> &mut u64 {
                &mut *self.tx.maturity_mut()
            }

            fn gas_price(&self) -> u64 {
                *self.tx.gas_price()
            }

            fn gas_price_mut(&mut self) -> &mut u64 {
                self.tx.gas_price_mut()
            }

            fn gas_limit(&self) -> u64 {
                *self.tx.gas_limit()
            }

            fn gas_limit_mut(&mut self) -> &mut u64 {
                self.tx.gas_limit_mut()
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

            fn outputs(&self) -> &Vec<Output> {
                self.tx.outputs()
            }

            fn outputs_mut(&mut self) -> &mut Vec<Output> {
                self.tx.outputs_mut()
            }

            fn witnesses(&self) -> &Vec<Witness> {
                self.tx.witnesses()
            }

            fn witnesses_mut(&mut self) -> &mut Vec<Witness> {
                self.tx.witnesses_mut()
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

    pub fn script_mut(&mut self) -> &mut Vec<u8> {
        self.tx.script_mut()
    }

    pub fn script_data(&self) -> &Vec<u8> {
        self.tx.script_data()
    }

    pub fn script_data_mut(&mut self) -> &mut Vec<u8> {
        self.tx.script_data_mut()
    }

    /// Craft a transaction used to transfer funds between two addresses.
    pub fn build_transfer_tx(
        inputs: &[Input],
        outputs: &[Output],
        params: TxParameters,
    ) -> ScriptTransaction {
        // This script is empty, since all this transaction does is move Inputs and Outputs around.
        FuelTransaction::script(
            params.gas_price,
            params.gas_limit,
            params.maturity,
            vec![],
            vec![],
            inputs.to_vec(),
            outputs.to_vec(),
            vec![],
        )
        .into()
    }

    /// Craft a transaction used to transfer funds to a contract.
    pub fn build_contract_transfer_tx(
        to: ContractId,
        amount: u64,
        asset_id: AssetId,
        inputs: &[Input],
        outputs: &[Output],
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
        inputs: &[Input],
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
            inputs.to_vec(),
            outputs.to_vec(),
            vec![],
        )
        .into()
    }
}
