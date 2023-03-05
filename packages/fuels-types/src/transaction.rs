use std::fmt::Debug;

use fuel_asm::{op, GTFArgs, RegId};
use fuel_tx::{
    Address, AssetId, Bytes32, ConsensusParameters, ContractId, Create, FormatValidityChecks,
    Input as FuelInput, Output, Script, StorageSlot, Transaction as FuelTransaction,
    TransactionFee, TxPointer, UniqueIdentifier, Witness,
};

use fuel_types::Salt;

use fuel_tx::field::GasLimit;
use fuel_tx::field::GasPrice;
use fuel_tx::field::Inputs;
use fuel_tx::field::Outputs;
use fuel_tx::field::Salt as FieldSalt;
use fuel_tx::field::Script as FieldScript;
use fuel_tx::field::ScriptData;
use fuel_tx::field::Witnesses;
use fuel_tx::field::{BytecodeLength, BytecodeWitnessIndex, Maturity, StorageSlots};
use fuel_tx::Chargeable;

use crate::coin::Coin;
use crate::input::Input;
use crate::message::Message;
use crate::resource::Resource;
use crate::transaction_builders::create_coin_input;
use crate::{
    constants::{BASE_ASSET_ID, WORD_SIZE},
    errors::Error,
    offsets,
    parameters::TxParameters,
};

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

    fn inputs(&self) -> &Vec<FuelInput>;

    fn outputs(&self) -> &Vec<Output>;

    fn witnesses(&self) -> &Vec<Witness>;
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

            fn inputs(&self) -> &Vec<FuelInput> {
                self.tx.inputs()
            }

            fn outputs(&self) -> &Vec<Output> {
                self.tx.outputs()
            }

            fn witnesses(&self) -> &Vec<Witness> {
                self.tx.witnesses()
            }
        }
    };
}

impl_tx_wrapper!(ScriptTransaction, Script);
impl_tx_wrapper!(CreateTransaction, Create);

impl CreateTransaction {
    pub fn salt(&self) -> &Salt {
        self.tx.salt()
    }

    pub fn bytecode_witness_index(&self) -> u8 {
        *self.tx.bytecode_witness_index()
    }

    pub fn storage_slots(&self) -> &Vec<StorageSlot> {
        self.tx.storage_slots()
    }

    pub fn bytecode_length(&self) -> u64 {
        *self.tx.bytecode_length()
    }
}

impl ScriptTransaction {
    pub fn script(&self) -> &Vec<u8> {
        self.tx.script()
    }
    pub fn script_data(&self) -> &Vec<u8> {
        self.tx.script_data()
    }
}
