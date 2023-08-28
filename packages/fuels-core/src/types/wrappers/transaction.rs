use std::{collections::HashSet, fmt::Debug};

use crate::types::{bech32::Bech32Address, coin::Coin, coin_type::CoinType};
use fuel_tx::{
    field::{
        GasLimit, GasPrice, Inputs, Maturity, Outputs, Script as ScriptField, ScriptData, Witnesses,
    },
    input::{
        coin::{CoinPredicate, CoinSigned},
        message::{MessageCoinPredicate, MessageCoinSigned},
    },
    Address, Bytes32, Chargeable, ConsensusParameters, Create, FormatValidityChecks, Input, Output,
    Salt as FuelSalt, Script, StorageSlot, Transaction as FuelTransaction, TransactionFee,
    UniqueIdentifier, UtxoId, Witness,
};
use fuel_types::ChainId;

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
    ($name: ident, $field: ident, $ty: ty) => {
        pub fn $name(mut self, $field: $ty) -> Self {
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

    impl_setter_getter!(with_gas_price, gas_price, u64);
    impl_setter_getter!(with_gas_limit, gas_limit, u64);
    impl_setter_getter!(with_maturity, maturity, u32);
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

use super::coin_type::CoinTypeId;

#[derive(Debug, Clone)]
pub enum TransactionType {
    Script(ScriptTransaction),
    Create(CreateTransaction),
}

#[derive(Clone, Debug)]
pub struct CachedTx {
    pub resource_ids_used: HashSet<CoinTypeId>,
    pub expected_resources: HashSet<CoinType>,
}

pub trait Transaction: Into<FuelTransaction> + Clone {
    fn compute_cached_tx(&self, address: &Bech32Address, chain_id: ChainId) -> CachedTx;

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

    fn with_tx_params(self, tx_params: TxParameters) -> Self;

    fn metered_bytes_size(&self) -> usize;

    fn inputs(&self) -> &Vec<Input>;

    fn outputs(&self) -> &Vec<Output>;

    fn witnesses(&self) -> &Vec<Witness>;

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
    fn compute_cached_tx(&self, address: &Bech32Address, chain_id: ChainId) -> CachedTx {
        match self {
            TransactionType::Script(tx) => tx.compute_cached_tx(address, chain_id),
            TransactionType::Create(tx) => tx.compute_cached_tx(address, chain_id),
        }
    }

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
}

fn extract_input_id(input: &Input, from_owner: Address) -> Option<CoinTypeId> {
    match input {
        Input::CoinSigned(CoinSigned { utxo_id, owner, .. })
        | Input::CoinPredicate(CoinPredicate { utxo_id, owner, .. })
            if (*owner == from_owner) =>
        {
            Some(CoinTypeId::UtxoId(*utxo_id))
        }
        Input::MessageCoinSigned(MessageCoinSigned {
            recipient, nonce, ..
        })
        | Input::MessageCoinPredicate(MessageCoinPredicate {
            recipient, nonce, ..
        }) if (*recipient == from_owner) => Some(CoinTypeId::Nonce(*nonce)),
        _ => None,
    }
}

fn extract_expected_coin(
    output: &Output,
    from_owner: Address,
    tx_id: Bytes32,
    idx: u8,
) -> Option<CoinType> {
    match output {
        Output::Coin {
            to,
            amount,
            asset_id,
        } if (*to == from_owner) => {
            let utxo_id = UtxoId::new(tx_id, idx);
            let coin = Coin::new_unspent(*amount, *asset_id, utxo_id, (*to).into());
            Some(CoinType::Coin(coin))
        }
        _ => None,
    }
}

macro_rules! impl_tx_wrapper {
    ($wrapper: ident, $wrapped: ident) => {
        #[derive(Debug, Clone)]
        pub struct $wrapper {
            pub(crate) tx: $wrapped,
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
                $wrapper { tx }
            }
        }

        impl Transaction for $wrapper {
            fn compute_cached_tx(&self, address: &Bech32Address, chain_id: ChainId) -> CachedTx {
                let plain_address: Address = address.into();
                let resource_ids_used = self
                    .inputs()
                    .iter()
                    .filter_map(|input| extract_input_id(input, plain_address))
                    .collect();

                let tx_id = self.id(chain_id);
                let expected_resources = self
                    .outputs()
                    .iter()
                    .enumerate()
                    .filter_map(|(idx, output)| {
                        extract_expected_coin(output, plain_address, tx_id, idx as u8)
                    })
                    .collect();

                CachedTx {
                    resource_ids_used,
                    expected_resources,
                }
            }

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
