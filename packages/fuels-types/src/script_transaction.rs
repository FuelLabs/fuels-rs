use std::fmt::Debug;

use fuel_tx::field::{
    GasLimit, GasPrice, Inputs, Maturity, Outputs, Script as ScriptField, ScriptData, Witnesses,
};
use fuel_tx::{
    Bytes32, Chargeable, ConsensusParameters, Create, FormatValidityChecks, Input, Output, Script,
    Transaction as FuelTransaction, TransactionFee, UniqueIdentifier, Witness,
};

use crate::errors::Error;

pub trait Transaction: Into<FuelTransaction> {
    fn fee_checked_from_tx(&self, params: &ConsensusParameters) -> Option<TransactionFee>;

    fn check_without_signatures(
        &self,
        block_height: u64,
        parameters: &ConsensusParameters,
    ) -> Result<(), Error>;

    fn id(&self) -> Bytes32;

    fn maturity(&self) -> u64;

    fn gas_price(&self) -> u64;

    fn gas_limit(&self) -> u64;

    fn gas_price_mut(&mut self) -> &mut u64;

    fn gas_limit_mut(&mut self) -> &mut u64;

    fn metered_bytes_size(&self) -> usize;

    fn inputs(&self) -> &Vec<Input>;

    fn outputs(&self) -> &Vec<Output>;

    fn inputs_mut(&mut self) -> &mut Vec<Input>;

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

            fn gas_price(&self) -> u64 {
                *self.tx.gas_price()
            }

            fn gas_limit(&self) -> u64 {
                *self.tx.gas_limit()
            }

            fn gas_price_mut(&mut self) -> &mut u64 {
                self.tx.gas_price_mut()
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

            fn outputs(&self) -> &Vec<Output> {
                self.tx.outputs()
            }

            fn inputs_mut(&mut self) -> &mut Vec<Input> {
                self.tx.inputs_mut()
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

    pub fn script_data(&self) -> &Vec<u8> {
        self.tx.script_data()
    }
}
