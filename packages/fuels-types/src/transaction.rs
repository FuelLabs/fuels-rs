#![cfg(feature = "std")]

use std::{collections::HashSet, fmt::Debug};

use fuel_tx::{
    field::{
        GasLimit, GasPrice, Inputs, Maturity, Outputs, Script as ScriptField, ScriptData, Witnesses,
    },
    Address, Bytes32, Chargeable, ConsensusParameters, Create, FormatValidityChecks,
    Input as FuelInput, Output, Salt as FuelSalt, Script, StorageSlot,
    Transaction as FuelTransaction, TransactionFee, UniqueIdentifier, UtxoId, Witness,
};

use crate::{
    bech32::Bech32Address,
    coin::Coin,
    constants::{DEFAULT_GAS_LIMIT, DEFAULT_GAS_PRICE, DEFAULT_MATURITY},
    errors::Error,
    resource::{Resource, ResourceId},
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
use fuel_tx::field::{BytecodeLength, BytecodeWitnessIndex, Salt, StorageSlots};

#[derive(Clone, Debug)]
pub struct CachedTx {
    pub resource_ids_used: HashSet<ResourceId>,
    pub expected_resources: HashSet<Resource>,
}

pub trait Transaction: Into<FuelTransaction> + Send {
    fn compute_cached_tx(&self, address: &Bech32Address) -> CachedTx;

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
            fn compute_cached_tx(&self, address: &Bech32Address) -> CachedTx {
                let plain_address: Address = address.into();
                let resource_ids_used = self
                    .inputs()
                    .iter()
                    .filter_map(|input| match input {
                        FuelInput::CoinSigned { utxo_id, owner, .. }
                        | FuelInput::CoinPredicate { utxo_id, owner, .. }
                            if (*owner == plain_address) =>
                        {
                            Some(ResourceId::UtxoId(utxo_id.clone()))
                        }
                        FuelInput::MessageSigned {
                            message_id,
                            recipient,
                            ..
                        }
                        | FuelInput::MessagePredicate {
                            message_id,
                            recipient,
                            ..
                        } if (*recipient == plain_address) => {
                            Some(ResourceId::MessageId(message_id.clone()))
                        }
                        _ => None,
                    })
                    .collect();

                let tx_id = self.id();
                let expected_resources = self
                    .outputs()
                    .iter()
                    .enumerate()
                    .filter_map(|(idx, output)| match output {
                        Output::Coin {
                            to,
                            amount,
                            asset_id,
                        } if (*to == plain_address) => {
                            let utxo_id = UtxoId::new(tx_id, idx as u8);
                            let coin = Coin::new_unspent(*amount, *asset_id, utxo_id, (*to).into());
                            Some(Resource::Coin(coin))
                        }
                        _ => None,
                    })
                    .collect();

                CachedTx {
                    resource_ids_used,
                    expected_resources,
                }
            }

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
