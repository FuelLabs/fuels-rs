use std::fmt::Debug;

use fuel_tx::{
    field::{
        GasLimit, GasPrice, Inputs, Maturity, Outputs, Script as ScriptField, ScriptData, Witnesses,
    },
    Bytes32, Chargeable, ConsensusParameters, Create, FormatValidityChecks, Input, Output,
    Salt as FuelSalt, Script, StorageSlot, Transaction as FuelTransaction, TransactionFee,
    UniqueIdentifier, Witness,
};

use crate::{
    constants::{DEFAULT_GAS_LIMIT, DEFAULT_GAS_PRICE, DEFAULT_MATURITY},
    types::Result,
};

#[derive(Debug, Copy, Clone)]
pub struct TxParameters {
    gas_price: u64,
    gas_limit: u64,
    maturity: u32,
}

macro_rules! impl_setter_getter {
    ($setter_name: ident, $field: ident, $ty: ty) => {
        pub fn $setter_name(mut self, $field: $ty) -> Self {
            self.$field = $field;
            self
        }

        pub fn $field(&self) -> $ty {
            self.$field
        }
    };
}

impl TxParameters {
    pub fn new(gas_price: u64, gas_limit: u64, maturity: u32) -> Self {
        Self {
            gas_price,
            gas_limit,
            maturity,
        }
    }

    impl_setter_getter!(set_gas_price, gas_price, u64);
    impl_setter_getter!(set_gas_limit, gas_limit, u64);
    impl_setter_getter!(set_maturity, maturity, u32);
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
use fuel_tx::field::{BytecodeLength, BytecodeWitnessIndex, Salt, StorageSlots};

#[derive(Debug, Clone)]
pub enum TransactionType {
    Script(ScriptTransaction),
    Create(CreateTransaction),
}

pub trait Transaction: Into<FuelTransaction> + Clone {
    fn id(&self) -> Bytes32;
    fn maturity(&self) -> u32;
    fn gas_price(&self) -> u64;
    fn gas_limit(&self) -> u64;
    fn inputs(&self) -> &Vec<Input>;
    fn outputs(&self) -> &Vec<Output>;
    fn witnesses(&self) -> &Vec<Witness>;
    fn append_witness(&mut self, witness: Witness) -> usize;
    fn metered_bytes_size(&self) -> usize;
    fn to_dry_run_tx(self, gas_price: u64, gas_limit: u64) -> Self;
    fn fee_checked_from_tx(&self) -> Option<TransactionFee>;
    fn check_without_signatures(
        &self,
        block_height: u32,
        parameters: &ConsensusParameters,
    ) -> Result<()>;
}

impl From<TransactionType> for FuelTransaction {
    fn from(value: TransactionType) -> Self {
        match value {
            TransactionType::Script(tx) => tx.into(),
            TransactionType::Create(tx) => tx.into(),
        }
    }
}

impl Transaction for TransactionType {
    fn id(&self) -> Bytes32 {
        match self {
            TransactionType::Script(tx) => tx.id(),
            TransactionType::Create(tx) => tx.id(),
        }
    }

    fn maturity(&self) -> u32 {
        match self {
            TransactionType::Script(tx) => tx.maturity(),
            TransactionType::Create(tx) => tx.maturity(),
        }
    }

    fn gas_price(&self) -> u64 {
        match self {
            TransactionType::Script(tx) => tx.gas_price(),
            TransactionType::Create(tx) => tx.gas_price(),
        }
    }

    fn gas_limit(&self) -> u64 {
        match self {
            TransactionType::Script(tx) => tx.gas_limit(),
            TransactionType::Create(tx) => tx.gas_limit(),
        }
    }

    fn inputs(&self) -> &Vec<Input> {
        match self {
            TransactionType::Script(tx) => tx.inputs(),
            TransactionType::Create(tx) => tx.inputs(),
        }
    }

    fn outputs(&self) -> &Vec<Output> {
        match self {
            TransactionType::Script(tx) => tx.outputs(),
            TransactionType::Create(tx) => tx.outputs(),
        }
    }

    fn witnesses(&self) -> &Vec<Witness> {
        match self {
            TransactionType::Script(tx) => tx.witnesses(),
            TransactionType::Create(tx) => tx.witnesses(),
        }
    }

    fn append_witness(&mut self, witness: Witness) -> usize {
        match self {
            TransactionType::Script(tx) => tx.append_witness(witness),
            TransactionType::Create(tx) => tx.append_witness(witness),
        }
    }

    fn metered_bytes_size(&self) -> usize {
        match self {
            TransactionType::Script(tx) => tx.metered_bytes_size(),
            TransactionType::Create(tx) => tx.metered_bytes_size(),
        }
    }

    fn to_dry_run_tx(self, gas_price: u64, gas_limit: u64) -> Self {
        match self {
            TransactionType::Script(tx) => {
                TransactionType::Script(tx.to_dry_run_tx(gas_price, gas_limit))
            }
            TransactionType::Create(tx) => {
                TransactionType::Create(tx.to_dry_run_tx(gas_price, gas_limit))
            }
        }
    }

    fn fee_checked_from_tx(&self) -> Option<TransactionFee> {
        match self {
            TransactionType::Script(tx) => tx.fee_checked_from_tx(),
            TransactionType::Create(tx) => tx.fee_checked_from_tx(),
        }
    }

    fn check_without_signatures(
        &self,
        block_height: u32,
        parameters: &ConsensusParameters,
    ) -> Result<()> {
        match self {
            TransactionType::Script(tx) => tx.check_without_signatures(block_height, parameters),
            TransactionType::Create(tx) => tx.check_without_signatures(block_height, parameters),
        }
    }
}

macro_rules! impl_tx_wrapper {
    ($wrapper: ident, $wrapped: ident) => {
        #[derive(Debug, Clone)]
        pub struct $wrapper {
            pub(crate) tx: $wrapped,
            pub(crate) consensus_parameters: ConsensusParameters,
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

        impl $wrapper {
            pub fn from_fuel_tx(tx: $wrapped, consensus_parameters: ConsensusParameters) -> Self {
                $wrapper {
                    tx,
                    consensus_parameters,
                }
            }
        }

        impl Transaction for $wrapper {
            fn id(&self) -> Bytes32 {
                self.tx.id(&self.consensus_parameters.chain_id.into())
            }

            fn maturity(&self) -> u32 {
                (*self.tx.maturity()).into()
            }

            fn gas_price(&self) -> u64 {
                *self.tx.gas_price()
            }

            fn gas_limit(&self) -> u64 {
                *self.tx.gas_limit()
            }

            fn inputs(&self) -> &Vec<Input> {
                self.tx.inputs()
            }

            fn outputs(&self) -> &Vec<Output> {
                self.tx.outputs()
            }

            fn witnesses(&self) -> &Vec<Witness> {
                self.tx.witnesses()
            }

            fn append_witness(&mut self, witness: Witness) -> usize {
                let idx = self.tx.witnesses().len();
                self.tx.witnesses_mut().push(witness);

                idx
            }

            fn metered_bytes_size(&self) -> usize {
                self.tx.metered_bytes_size()
            }

            fn to_dry_run_tx(mut self, gas_price: u64, gas_limit: u64) -> Self {
                *self.tx.gas_price_mut() = gas_price;
                *self.tx.gas_limit_mut() = gas_limit;

                self
            }

            fn fee_checked_from_tx(&self) -> Option<TransactionFee> {
                TransactionFee::checked_from_tx(&self.consensus_parameters, &self.tx)
            }

            fn check_without_signatures(
                &self,
                block_height: u32,
                parameters: &ConsensusParameters,
            ) -> Result<()> {
                Ok(self
                    .tx
                    .check_without_signatures(block_height.into(), parameters)?)
            }
        }
    };
}

impl_tx_wrapper!(ScriptTransaction, Script);
impl_tx_wrapper!(CreateTransaction, Create);

impl CreateTransaction {
    pub fn salt(&self) -> &FuelSalt {
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
