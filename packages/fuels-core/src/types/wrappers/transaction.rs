use std::fmt::Debug;

use fuel_tx::{
    field::{
        GasLimit, GasPrice, Inputs, Maturity, Outputs, Script as ScriptField, ScriptData, Witnesses,
    },
    Bytes32, Cacheable, Chargeable, ConsensusParameters, Create, FormatValidityChecks,
    Input as FuelInput, Output, Salt as FuelSalt, Script, StorageSlot,
    Transaction as FuelTransaction, TransactionFee, UniqueIdentifier, Witness,
};
use fuel_vm::{checked_transaction::EstimatePredicates, gas::GasCosts};

use crate::{
    constants::{DEFAULT_GAS_LIMIT, DEFAULT_GAS_PRICE, DEFAULT_MATURITY},
    types::errors::Error,
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

pub trait Transaction: Into<FuelTransaction> + Send {
    fn fee_checked_from_tx(&self, params: &ConsensusParameters) -> Option<TransactionFee>;

    fn check_without_signatures(
        &self,
        block_height: u32,
        parameters: &ConsensusParameters,
    ) -> Result<(), Error>;

    fn precompute(&mut self, chain_id: u64) -> Result<(), Error>;

    fn is_computed(&self) -> bool;

    fn estimate_predicates(&mut self, parameters: &ConsensusParameters) -> Result<(), Error>;

    fn id(&self, chain_id: u64) -> Bytes32;

    fn maturity(&self) -> u32;

    fn with_maturity(self, maturity: u32) -> Self;

    fn gas_price(&self) -> u64;

    fn with_gas_price(self, gas_price: u64) -> Self;

    fn gas_limit(&self) -> u64;

    fn with_gas_limit(self, gas_limit: u64) -> Self;

    fn with_tx_params(self, tx_params: TxParameters) -> Self;

    fn metered_bytes_size(&self) -> usize;

    fn inputs(&self) -> &Vec<FuelInput>;

    fn outputs(&self) -> &Vec<Output>;

    fn witnesses(&self) -> &Vec<Witness>;

    fn witnesses_mut(&mut self) -> &mut Vec<Witness>;

    fn with_witnesses(self, witnesses: Vec<Witness>) -> Self;
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
    fn fee_checked_from_tx(&self, params: &ConsensusParameters) -> Option<TransactionFee> {
        match self {
            TransactionType::Script(tx) => tx.fee_checked_from_tx(params),
            TransactionType::Create(tx) => tx.fee_checked_from_tx(params),
        }
    }

    fn is_computed(&self) -> bool {
        match self {
            TransactionType::Script(tx) => tx.is_computed(),
            TransactionType::Create(tx) => tx.is_computed(),
        }
    }

    fn precompute(&mut self, chain_id: u64) -> Result<(), Error> {
        match self {
            TransactionType::Script(tx) => tx.precompute(chain_id)?,
            TransactionType::Create(tx) => tx.precompute(chain_id)?,
        }
        Ok(())
    }

    fn estimate_predicates(&mut self, parameters: &ConsensusParameters) -> Result<(), Error> {
        match self {
            TransactionType::Script(tx) => tx.estimate_predicates(parameters)?,
            TransactionType::Create(tx) => tx.estimate_predicates(parameters)?,
        };
        Ok(())
    }

    fn check_without_signatures(
        &self,
        block_height: u32,
        parameters: &ConsensusParameters,
    ) -> Result<(), Error> {
        match self {
            TransactionType::Script(tx) => tx.check_without_signatures(block_height, parameters),
            TransactionType::Create(tx) => tx.check_without_signatures(block_height, parameters),
        }
    }

    fn id(&self, chain_id: u64) -> Bytes32 {
        match self {
            TransactionType::Script(tx) => tx.id(chain_id),
            TransactionType::Create(tx) => tx.id(chain_id),
        }
    }

    fn maturity(&self) -> u32 {
        match self {
            TransactionType::Script(tx) => tx.maturity(),
            TransactionType::Create(tx) => tx.maturity(),
        }
    }

    fn with_maturity(self, maturity: u32) -> Self {
        match self {
            TransactionType::Script(tx) => TransactionType::Script(tx.with_maturity(maturity)),
            TransactionType::Create(tx) => TransactionType::Create(tx.with_maturity(maturity)),
        }
    }

    fn gas_price(&self) -> u64 {
        match self {
            TransactionType::Script(tx) => tx.gas_price(),
            TransactionType::Create(tx) => tx.gas_price(),
        }
    }

    fn with_gas_price(self, gas_price: u64) -> Self {
        match self {
            TransactionType::Script(tx) => TransactionType::Script(tx.with_gas_price(gas_price)),
            TransactionType::Create(tx) => TransactionType::Create(tx.with_gas_price(gas_price)),
        }
    }

    fn gas_limit(&self) -> u64 {
        match self {
            TransactionType::Script(tx) => tx.gas_limit(),
            TransactionType::Create(tx) => tx.gas_limit(),
        }
    }

    fn with_gas_limit(self, gas_limit: u64) -> Self {
        match self {
            TransactionType::Script(tx) => TransactionType::Script(tx.with_gas_limit(gas_limit)),
            TransactionType::Create(tx) => TransactionType::Create(tx.with_gas_limit(gas_limit)),
        }
    }

    fn with_tx_params(self, tx_params: TxParameters) -> Self {
        match self {
            TransactionType::Script(tx) => TransactionType::Script(tx.with_tx_params(tx_params)),
            TransactionType::Create(tx) => TransactionType::Create(tx.with_tx_params(tx_params)),
        }
    }

    fn metered_bytes_size(&self) -> usize {
        match self {
            TransactionType::Script(tx) => tx.metered_bytes_size(),
            TransactionType::Create(tx) => tx.metered_bytes_size(),
        }
    }

    fn inputs(&self) -> &Vec<FuelInput> {
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

    fn witnesses_mut(&mut self) -> &mut Vec<Witness> {
        match self {
            TransactionType::Script(tx) => tx.witnesses_mut(),
            TransactionType::Create(tx) => tx.witnesses_mut(),
        }
    }

    fn with_witnesses(self, witnesses: Vec<Witness>) -> Self {
        match self {
            TransactionType::Script(tx) => TransactionType::Script(tx.with_witnesses(witnesses)),
            TransactionType::Create(tx) => TransactionType::Create(tx.with_witnesses(witnesses)),
        }
    }
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

        impl AsMut<$wrapped> for $wrapper {
            fn as_mut(&mut self) -> &mut $wrapped {
                &mut self.tx
            }
        }

        impl Transaction for $wrapper {
            fn fee_checked_from_tx(&self, params: &ConsensusParameters) -> Option<TransactionFee> {
                TransactionFee::checked_from_tx(params, &self.tx)
            }

            fn check_without_signatures(
                &self,
                block_height: u32,
                parameters: &ConsensusParameters,
            ) -> Result<(), Error> {
                Ok(self
                    .tx
                    .check_without_signatures(block_height.into(), parameters)?)
            }

            fn is_computed(&self) -> bool {
                self.tx.is_computed()
            }

            // TODO: Fetch `GasCosts` from the `fuel-core`:
            //  https://github.com/FuelLabs/fuel-core/issues/1221
            fn estimate_predicates(
                &mut self,
                parameters: &ConsensusParameters,
            ) -> Result<(), Error> {
                self.tx
                    .estimate_predicates(parameters, &GasCosts::default())
                    .map_err(Error::ValidationError)
            }

            fn precompute(&mut self, chain_id: u64) -> Result<(), Error> {
                Ok(self.tx.precompute(&chain_id.into())?)
            }

            fn id(&self, chain_id: u64) -> Bytes32 {
                self.tx.id(&chain_id.into())
            }

            fn maturity(&self) -> u32 {
                (*self.tx.maturity()).into()
            }

            fn with_maturity(mut self, maturity: u32) -> Self {
                *self.tx.maturity_mut() = maturity.into();
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
