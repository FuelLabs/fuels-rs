use std::fmt::Debug;

use fuel_tx::{
    field::{
        GasLimit, GasPrice, Inputs, Maturity, Outputs, Script as ScriptField, ScriptData, Witnesses,
    },
    Bytes32, Cacheable, Chargeable, ConsensusParameters, Create, FormatValidityChecks, Input,
    Output, Salt as FuelSalt, Script, StorageSlot, Transaction as FuelTransaction, TransactionFee,
    UniqueIdentifier, Witness,
};

use fuel_types::ChainId;
use fuel_vm::{checked_transaction::EstimatePredicates, prelude::GasCosts};

use crate::types::Result;

#[derive(Default, Debug, Copy, Clone)]
pub struct TxParameters {
    gas_price: Option<u64>,
    gas_limit: Option<u64>,
    maturity: u32,
}

impl TxParameters {
    pub fn new(gas_price: Option<u64>, gas_limit: Option<u64>, maturity: u32) -> Self {
        Self {
            gas_price,
            gas_limit,
            maturity,
        }
    }

    pub fn with_gas_price(mut self, gas_price: u64) -> Self {
        self.gas_price = Some(gas_price);
        self
    }

    pub fn gas_price(&self) -> Option<u64> {
        self.gas_price
    }

    pub fn with_gas_limit(mut self, gas_limit: u64) -> Self {
        self.gas_limit = Some(gas_limit);
        self
    }

    pub fn gas_limit(&self) -> Option<u64> {
        self.gas_limit
    }

    pub fn with_maturity(mut self, maturity: u32) -> Self {
        self.maturity = maturity;
        self
    }

    pub fn maturity(&self) -> u32 {
        self.maturity
    }
}

use fuel_tx::field::{BytecodeLength, BytecodeWitnessIndex, Salt, StorageSlots};

#[derive(Debug, Clone)]
pub enum TransactionType {
    Script(ScriptTransaction),
    Create(CreateTransaction),
}

pub trait Transaction: Into<FuelTransaction> + Clone {
    fn fee_checked_from_tx(
        &self,
        consensus_parameters: &ConsensusParameters,
    ) -> Option<TransactionFee>;

    fn check_without_signatures(
        &self,
        block_height: u32,
        consensus_parameters: &ConsensusParameters,
    ) -> Result<()>;

    fn id(&self, chain_id: ChainId) -> Bytes32;

    fn maturity(&self) -> u32;

    fn with_maturity(self, maturity: u32) -> Self;

    fn gas_price(&self) -> u64;

    fn with_gas_price(self, gas_price: u64) -> Self;

    fn gas_limit(&self) -> u64;

    fn with_gas_limit(self, gas_limit: u64) -> Self;

    fn metered_bytes_size(&self) -> usize;

    fn inputs(&self) -> &Vec<Input>;

    fn outputs(&self) -> &Vec<Output>;

    fn witnesses(&self) -> &Vec<Witness>;

    fn is_using_predicates(&self) -> bool;

    /// Precompute transaction metadata. The metadata is required for
    /// `check_without_signatures` validation.
    fn precompute(&mut self, chain_id: &ChainId) -> Result<()>;

    /// If a transactions contains predicates, we have to estimate them
    /// before sending the transaction to the node. The estimation will check
    /// all predicates and set the `predicate_gas_used` to the actual consumed gas.
    fn estimate_predicates(
        &mut self,
        consensus_parameters: &ConsensusParameters,
        gas_costs: &GasCosts,
    ) -> Result<()>;

    /// Append witness and return the corresponding witness index
    fn append_witness(&mut self, witness: Witness) -> usize;
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
    fn fee_checked_from_tx(
        &self,
        consensus_parameters: &ConsensusParameters,
    ) -> Option<TransactionFee> {
        match self {
            TransactionType::Script(tx) => tx.fee_checked_from_tx(consensus_parameters),
            TransactionType::Create(tx) => tx.fee_checked_from_tx(consensus_parameters),
        }
    }

    fn check_without_signatures(
        &self,
        block_height: u32,
        consensus_parameters: &ConsensusParameters,
    ) -> Result<()> {
        match self {
            TransactionType::Script(tx) => {
                tx.check_without_signatures(block_height, consensus_parameters)
            }
            TransactionType::Create(tx) => {
                tx.check_without_signatures(block_height, consensus_parameters)
            }
        }
    }

    fn id(&self, chain_id: ChainId) -> Bytes32 {
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

    fn metered_bytes_size(&self) -> usize {
        match self {
            TransactionType::Script(tx) => tx.metered_bytes_size(),
            TransactionType::Create(tx) => tx.metered_bytes_size(),
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

    fn is_using_predicates(&self) -> bool {
        match self {
            TransactionType::Script(tx) => tx.is_using_predicates(),
            TransactionType::Create(tx) => tx.is_using_predicates(),
        }
    }

    fn precompute(&mut self, chain_id: &ChainId) -> Result<()> {
        match self {
            TransactionType::Script(tx) => tx.precompute(chain_id),
            TransactionType::Create(tx) => tx.precompute(chain_id),
        }
    }

    fn estimate_predicates(
        &mut self,
        consensus_parameters: &ConsensusParameters,
        gas_costs: &GasCosts,
    ) -> Result<()> {
        match self {
            TransactionType::Script(tx) => tx.estimate_predicates(consensus_parameters, gas_costs),
            TransactionType::Create(tx) => tx.estimate_predicates(consensus_parameters, gas_costs),
        }
    }

    fn append_witness(&mut self, witness: Witness) -> usize {
        match self {
            TransactionType::Script(tx) => tx.append_witness(witness),
            TransactionType::Create(tx) => tx.append_witness(witness),
        }
    }
}

macro_rules! impl_tx_wrapper {
    ($wrapper: ident, $wrapped: ident) => {
        #[derive(Debug, Clone)]
        pub struct $wrapper {
            pub(crate) tx: $wrapped,
            pub(crate) is_using_predicates: bool,
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

        impl From<$wrapped> for $wrapper {
            fn from(tx: $wrapped) -> Self {
                let is_using_predicates = tx.inputs().iter().any(|input| {
                    matches!(
                        input,
                        Input::CoinPredicate { .. }
                            | Input::MessageCoinPredicate { .. }
                            | Input::MessageDataPredicate { .. }
                    )
                });

                $wrapper {
                    tx,
                    is_using_predicates,
                }
            }
        }

        impl Transaction for $wrapper {
            fn fee_checked_from_tx(
                &self,
                consensus_parameters: &ConsensusParameters,
            ) -> Option<TransactionFee> {
                TransactionFee::checked_from_tx(consensus_parameters, &self.tx)
            }

            fn check_without_signatures(
                &self,
                block_height: u32,
                consensus_parameters: &ConsensusParameters,
            ) -> Result<()> {
                Ok(self
                    .tx
                    .check_without_signatures(block_height.into(), consensus_parameters)?)
            }

            fn id(&self, chain_id: ChainId) -> Bytes32 {
                self.tx.id(&chain_id)
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

            fn metered_bytes_size(&self) -> usize {
                self.tx.metered_bytes_size()
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

            fn is_using_predicates(&self) -> bool {
                self.is_using_predicates
            }

            fn precompute(&mut self, chain_id: &ChainId) -> Result<()> {
                Ok(self.tx.precompute(chain_id)?)
            }

            fn estimate_predicates(
                &mut self,
                consensus_parameters: &ConsensusParameters,
                gas_costs: &GasCosts,
            ) -> Result<()> {
                let gas_price = *self.tx.gas_price();
                let gas_limit = *self.tx.gas_limit();
                *self.tx.gas_price_mut() = 0;
                *self.tx.gas_limit_mut() = consensus_parameters.max_gas_per_tx;

                self.tx
                    .estimate_predicates(consensus_parameters, gas_costs)?;
                *self.tx.gas_price_mut() = gas_price;
                *self.tx.gas_limit_mut() = gas_limit;

                Ok(())
            }

            fn append_witness(&mut self, witness: Witness) -> usize {
                let idx = self.tx.witnesses().len();
                self.tx.witnesses_mut().push(witness);

                idx
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
