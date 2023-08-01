use std::fmt::Debug;

use fuel_crypto::{Message, SecretKey, Signature};
use fuel_tx::{
    field::{
        GasLimit, GasPrice, Inputs, Maturity, Outputs, Script as ScriptField, ScriptData, Witnesses,
    },
    Bytes32, Cacheable, Chargeable, ConsensusParameters, Create, FormatValidityChecks, Input,
    Input as FuelInput, Output, Salt as FuelSalt, Script, StorageSlot,
    Transaction as FuelTransaction, TransactionFee, UniqueIdentifier, Witness,
};
use fuel_vm::{checked_transaction::EstimatePredicates, gas::GasCosts};

use crate::{
    constants::{DEFAULT_GAS_LIMIT, DEFAULT_GAS_PRICE, DEFAULT_MATURITY},
    types::{bech32::Bech32Address, Address, Result},
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
    fn add_unresolved_signature(&mut self, owner: &Bech32Address, secret_key: SecretKey);

    fn resolve_transaction(&mut self) -> Result<()>;

    fn fee_checked_from_tx(&self) -> Option<TransactionFee>;

    fn check_without_signatures(
        &self,
        block_height: u32,
        parameters: &ConsensusParameters,
    ) -> Result<()>;

    fn to_dry_run_tx(self, gas_price: u64, gas_limit: u64) -> Self;

    fn id(&self) -> Bytes32;

    fn maturity(&self) -> u32;

    fn gas_price(&self) -> u64;

    fn gas_limit(&self) -> u64;

    fn metered_bytes_size(&self) -> usize;

    fn inputs(&self) -> &Vec<FuelInput>;

    fn outputs(&self) -> &Vec<Output>;

    fn witnesses(&self) -> &Vec<Witness>;
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
    fn add_unresolved_signature(&mut self, owner: &Bech32Address, secret_key: SecretKey) {
        match self {
            TransactionType::Script(tx) => tx.add_unresolved_signature(owner, secret_key),
            TransactionType::Create(tx) => tx.add_unresolved_signature(owner, secret_key),
        }
    }

    fn resolve_transaction(&mut self) -> Result<()> {
        match self {
            TransactionType::Script(tx) => tx.resolve_transaction(),
            TransactionType::Create(tx) => tx.resolve_transaction(),
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
}

#[derive(Debug, Clone)]
pub(crate) struct UnresolvedSignature {
    owner: Address,
    secret_key: SecretKey,
}

macro_rules! impl_tx_wrapper {
    ($wrapper: ident, $wrapped: ident) => {
        #[derive(Debug, Clone)]
        pub struct $wrapper {
            pub(crate) tx: $wrapped,
            pub(crate) unresolved_signatures: Vec<UnresolvedSignature>,
            pub(crate) consensus_parameters: ConsensusParameters,
        }

        impl From<$wrapped> for $wrapper {
            fn from(tx: $wrapped) -> Self {
                $wrapper {
                    tx,
                    unresolved_signatures: Default::default(),
                    consensus_parameters: Default::default(),
                }
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

        impl $wrapper {
            fn update_witness_indexes(&mut self) {
                let current_index = self.tx.witnesses().len();

                for (new_index, UnresolvedSignature { owner, .. }) in
                    self.unresolved_signatures.iter().enumerate()
                {
                    for input in self.tx.inputs_mut() {
                        Self::update_witness_index_if_owner(
                            input,
                            owner,
                            (current_index + new_index) as u8,
                        );
                    }
                }
            }

            fn update_witness_index_if_owner(input: &mut Input, owner: &Address, index: u8) {
                match input {
                    FuelInput::CoinSigned(ref mut cs) if cs.owner == *owner => {
                        cs.witness_index = index
                    }
                    FuelInput::MessageCoinSigned(ref mut mcs) if mcs.recipient == *owner => {
                        mcs.witness_index = index
                    }
                    FuelInput::MessageDataSigned(ref mut mds) if mds.recipient == *owner => {
                        mds.witness_index = index
                    }
                    _ => (),
                }
            }

            fn add_missing_witnesses(&mut self) {
                let id = self.id();
                let new_witnesses = self.unresolved_signatures.iter().map(
                    |UnresolvedSignature { secret_key, .. }| {
                        let message = Message::from_bytes(*id);
                        let signature = Signature::sign(secret_key, &message);

                        Witness::from(signature.as_ref())
                    },
                );

                self.tx.witnesses_mut().extend(new_witnesses);
            }
        }

        impl Transaction for $wrapper {
            fn add_unresolved_signature(&mut self, owner: &Bech32Address, secret_key: SecretKey) {
                self.unresolved_signatures.push(UnresolvedSignature {
                    owner: owner.into(),
                    secret_key,
                })
            }

            fn resolve_transaction(&mut self) -> Result<()> {
                self.update_witness_indexes();
                self.add_missing_witnesses();

                self.tx.precompute(&self.consensus_parameters.chain_id)?;

                // TODO: Fetch `GasCosts` from the `fuel-core`:
                //  https://github.com/FuelLabs/fuel-core/issues/1221
                self.tx
                    .estimate_predicates(&self.consensus_parameters, &GasCosts::default())?;

                Ok(())
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

            fn to_dry_run_tx(mut self, gas_price: u64, gas_limit: u64) -> Self {
                *self.tx.gas_price_mut() = gas_price;
                *self.tx.gas_limit_mut() = gas_limit;

                self
            }

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
