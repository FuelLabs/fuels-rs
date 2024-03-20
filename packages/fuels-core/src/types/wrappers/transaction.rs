use std::{collections::HashMap, fmt::Debug};

use async_trait::async_trait;
use fuel_crypto::{Message, Signature};
use fuel_tx::{
    field::{
        Inputs, Maturity, MintAmount, MintAssetId, Outputs, Script as ScriptField, ScriptData,
        ScriptGasLimit, WitnessLimit, Witnesses,
    },
    input::{
        coin::{CoinPredicate, CoinSigned},
        message::{
            MessageCoinPredicate, MessageCoinSigned, MessageDataPredicate, MessageDataSigned,
        },
    },
    Buildable, Bytes32, Cacheable, Chargeable, ConsensusParameters, Create, FormatValidityChecks,
    Input, Mint, Output, Salt as FuelSalt, Script, StorageSlot, Transaction as FuelTransaction,
    TransactionFee, UniqueIdentifier, Witness,
};
use fuel_types::{bytes::padded_len_usize, AssetId, ChainId};
use fuel_vm::checked_transaction::{
    CheckPredicateParams, CheckPredicates, EstimatePredicates, IntoChecked,
};
use itertools::Itertools;

use crate::{
    constants::BASE_ASSET_ID,
    traits::Signer,
    types::{
        bech32::Bech32Address,
        errors::{error_transaction, Result},
    },
    utils::{calculate_witnesses_size, sealed},
};

#[derive(Default, Debug, Clone)]
pub struct Transactions {
    fuel_transactions: Vec<FuelTransaction>,
}

impl Transactions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(mut self, tx: impl Into<FuelTransaction>) -> Self {
        self.fuel_transactions.push(tx.into());

        self
    }

    pub fn as_slice(&self) -> &[FuelTransaction] {
        &self.fuel_transactions
    }
}

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct MintTransaction {
    tx: Box<Mint>,
}

impl From<MintTransaction> for FuelTransaction {
    fn from(mint: MintTransaction) -> Self {
        (*mint.tx).into()
    }
}

impl From<MintTransaction> for Mint {
    fn from(tx: MintTransaction) -> Self {
        *tx.tx
    }
}

impl From<Mint> for MintTransaction {
    fn from(tx: Mint) -> Self {
        Self { tx: Box::new(tx) }
    }
}

impl MintTransaction {
    pub fn check_without_signatures(
        &self,
        block_height: u32,
        consensus_parameters: &ConsensusParameters,
    ) -> Result<()> {
        Ok(self
            .tx
            .check_without_signatures(block_height.into(), consensus_parameters)?)
    }
    #[must_use]
    pub fn id(&self, chain_id: ChainId) -> Bytes32 {
        self.tx.id(&chain_id)
    }

    #[must_use]
    pub fn mint_asset_id(&self) -> &AssetId {
        self.tx.mint_asset_id()
    }

    #[must_use]
    pub fn mint_amount(&self) -> u64 {
        *self.tx.mint_amount()
    }
}

#[derive(Default, Debug, Copy, Clone)]
//ANCHOR: tx_policies_struct
pub struct TxPolicies {
    tip: Option<u64>,
    witness_limit: Option<u64>,
    maturity: Option<u64>,
    max_fee: Option<u64>,
    script_gas_limit: Option<u64>,
}
//ANCHOR_END: tx_policies_struct

impl TxPolicies {
    pub fn new(
        tip: Option<u64>,
        witness_limit: Option<u64>,
        maturity: Option<u64>,
        max_fee: Option<u64>,
        script_gas_limit: Option<u64>,
    ) -> Self {
        Self {
            tip,
            witness_limit,
            maturity,
            max_fee,
            script_gas_limit,
        }
    }

    pub fn with_tip(mut self, tip: u64) -> Self {
        self.tip = Some(tip);
        self
    }

    pub fn tip(&self) -> Option<u64> {
        self.tip
    }

    pub fn with_witness_limit(mut self, witness_limit: u64) -> Self {
        self.witness_limit = Some(witness_limit);
        self
    }

    pub fn witness_limit(&self) -> Option<u64> {
        self.witness_limit
    }

    pub fn with_maturity(mut self, maturity: u64) -> Self {
        self.maturity = Some(maturity);
        self
    }

    pub fn maturity(&self) -> Option<u64> {
        self.maturity
    }

    pub fn with_max_fee(mut self, max_fee: u64) -> Self {
        self.max_fee = Some(max_fee);
        self
    }

    pub fn max_fee(&self) -> Option<u64> {
        self.max_fee
    }

    pub fn with_script_gas_limit(mut self, script_gas_limit: u64) -> Self {
        self.script_gas_limit = Some(script_gas_limit);
        self
    }

    pub fn script_gas_limit(&self) -> Option<u64> {
        self.script_gas_limit
    }
}

use fuel_tx::field::{BytecodeLength, BytecodeWitnessIndex, Salt, StorageSlots};

use crate::types::coin_type_id::CoinTypeId;

#[derive(Debug, Clone)]
pub enum TransactionType {
    Script(ScriptTransaction),
    Create(CreateTransaction),
    Mint(MintTransaction),
}

pub trait EstimablePredicates: sealed::Sealed {
    /// If a transaction contains predicates, we have to estimate them
    /// before sending the transaction to the node. The estimation will check
    /// all predicates and set the `predicate_gas_used` to the actual consumed gas.
    fn estimate_predicates(&mut self, consensus_parameters: &ConsensusParameters) -> Result<()>;
}

pub trait GasValidation: sealed::Sealed {
    fn validate_gas(&self, _gas_used: u64) -> Result<()>;
}

pub trait ValidatablePredicates: sealed::Sealed {
    /// If a transaction contains predicates, we can verify that these predicates validate, ie
    /// that they return `true`
    fn validate_predicates(
        self,
        consensus_parameters: &ConsensusParameters,
        block_height: u32,
    ) -> Result<()>;
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait Transaction:
    Into<FuelTransaction>
    + EstimablePredicates
    + ValidatablePredicates
    + GasValidation
    + Clone
    + Debug
    + sealed::Sealed
{
    fn fee_checked_from_tx(
        &self,
        consensus_parameters: &ConsensusParameters,
        gas_price: u64,
    ) -> Option<TransactionFee>;

    fn max_gas(&self, consensus_parameters: &ConsensusParameters) -> u64;

    /// Performs all stateless transaction validity checks. This includes the validity
    /// of fields according to rules in the specification and validity of signatures.
    /// <https://github.com/FuelLabs/fuel-specs/blob/master/src/tx-format/transaction.md>
    fn check(&self, block_height: u32, consensus_parameters: &ConsensusParameters) -> Result<()>;

    fn id(&self, chain_id: ChainId) -> Bytes32;

    fn maturity(&self) -> u32;

    fn with_maturity(self, maturity: u32) -> Self;

    fn metered_bytes_size(&self) -> usize;

    fn inputs(&self) -> &Vec<Input>;

    fn outputs(&self) -> &Vec<Output>;

    fn witnesses(&self) -> &Vec<Witness>;

    fn is_using_predicates(&self) -> bool;

    /// Precompute transaction metadata. The metadata is required for
    /// `check_without_signatures` validation.
    fn precompute(&mut self, chain_id: &ChainId) -> Result<()>;

    /// Append witness and return the corresponding witness index
    fn append_witness(&mut self, witness: Witness) -> Result<usize>;

    fn used_coins(&self) -> HashMap<(Bech32Address, AssetId), Vec<CoinTypeId>>;

    async fn sign_with(
        &mut self,
        signer: &(impl Signer + Send + Sync),
        chain_id: ChainId,
    ) -> Result<Signature>;
}

impl From<TransactionType> for FuelTransaction {
    fn from(value: TransactionType) -> Self {
        match value {
            TransactionType::Script(tx) => tx.into(),
            TransactionType::Create(tx) => tx.into(),
            TransactionType::Mint(tx) => tx.into(),
        }
    }
}

fn extract_coin_type_id(input: &Input) -> Option<CoinTypeId> {
    if let Some(utxo_id) = input.utxo_id() {
        return Some(CoinTypeId::UtxoId(*utxo_id));
    } else if let Some(nonce) = input.nonce() {
        return Some(CoinTypeId::Nonce(*nonce));
    }

    None
}

pub fn extract_owner_or_recipient(input: &Input) -> Option<Bech32Address> {
    let addr = match input {
        Input::CoinSigned(CoinSigned { owner, .. })
        | Input::CoinPredicate(CoinPredicate { owner, .. }) => Some(owner),
        Input::MessageCoinSigned(MessageCoinSigned { recipient, .. })
        | Input::MessageCoinPredicate(MessageCoinPredicate { recipient, .. })
        | Input::MessageDataSigned(MessageDataSigned { recipient, .. })
        | Input::MessageDataPredicate(MessageDataPredicate { recipient, .. }) => Some(recipient),
        Input::Contract(_) => None,
    };

    addr.map(|addr| Bech32Address::from(*addr))
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

        impl ValidatablePredicates for $wrapper {
            fn validate_predicates(
                self,
                consensus_parameters: &ConsensusParameters,
                block_height: u32,
            ) -> Result<()> {
                let checked = self
                    .tx
                    .into_checked(block_height.into(), consensus_parameters)?;
                let check_predicates_parameters: CheckPredicateParams = consensus_parameters.into();
                checked.check_predicates(&check_predicates_parameters)?;

                Ok(())
            }
        }

        impl sealed::Sealed for $wrapper {}

        #[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
        #[cfg_attr(not(target_arch = "wasm32"), async_trait)]
        impl Transaction for $wrapper {
            fn max_gas(&self, consensus_parameters: &ConsensusParameters) -> u64 {
                self.tx.max_gas(
                    consensus_parameters.gas_costs(),
                    consensus_parameters.fee_params(),
                )
            }

            fn fee_checked_from_tx(
                &self,
                consensus_parameters: &ConsensusParameters,
                gas_price: u64,
            ) -> Option<TransactionFee> {
                TransactionFee::checked_from_tx(
                    &consensus_parameters.gas_costs,
                    consensus_parameters.fee_params(),
                    &self.tx,
                    gas_price,
                )
            }

            fn check(
                &self,
                block_height: u32,
                consensus_parameters: &ConsensusParameters,
            ) -> Result<()> {
                Ok(self.tx.check(block_height.into(), consensus_parameters)?)
            }

            fn id(&self, chain_id: ChainId) -> Bytes32 {
                self.tx.id(&chain_id)
            }

            fn maturity(&self) -> u32 {
                (*self.tx.maturity()).into()
            }

            fn with_maturity(mut self, maturity: u32) -> Self {
                self.tx.set_maturity(maturity.into());
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

            fn append_witness(&mut self, witness: Witness) -> Result<usize> {
                let new_witnesses_size = padded_len_usize(calculate_witnesses_size(
                    self.tx.witnesses().iter().chain(std::iter::once(&witness)),
                )) as u64;

                if new_witnesses_size > self.tx.witness_limit() {
                    Err(error_transaction!(
                        Validation,
                        "Witness limit exceeded. Consider setting the limit manually with \
                        a transaction builder. The new limit should be: `{new_witnesses_size}`"
                    ))
                } else {
                    let idx = self.tx.witnesses().len();
                    self.tx.witnesses_mut().push(witness);

                    Ok(idx)
                }
            }

            fn used_coins(&self) -> HashMap<(Bech32Address, AssetId), Vec<CoinTypeId>> {
                self.inputs()
                    .iter()
                    .filter_map(|input| match input {
                        Input::Contract { .. } => None,
                        _ => {
                            // Not a contract, it's safe to expect.
                            let owner = extract_owner_or_recipient(input).expect("has owner");
                            let asset_id = input
                                .asset_id(&BASE_ASSET_ID)
                                .expect("has `asset_id`")
                                .to_owned();

                            let id = extract_coin_type_id(input).unwrap();
                            Some(((owner, asset_id), id))
                        }
                    })
                    .into_group_map()
            }

            async fn sign_with(
                &mut self,
                signer: &(impl Signer + Send + Sync),
                chain_id: ChainId,
            ) -> Result<Signature> {
                let tx_id = self.id(chain_id);
                let message = Message::from_bytes(*tx_id);
                let signature = signer.sign(message).await?;

                self.append_witness(signature.as_ref().into())?;

                Ok(signature)
            }
        }
    };
}

impl_tx_wrapper!(ScriptTransaction, Script);
impl_tx_wrapper!(CreateTransaction, Create);

impl EstimablePredicates for CreateTransaction {
    fn estimate_predicates(&mut self, consensus_parameters: &ConsensusParameters) -> Result<()> {
        self.tx.estimate_predicates(&consensus_parameters.into())?;

        Ok(())
    }
}

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

impl EstimablePredicates for ScriptTransaction {
    fn estimate_predicates(&mut self, consensus_parameters: &ConsensusParameters) -> Result<()> {
        self.tx.estimate_predicates(&consensus_parameters.into())?;

        Ok(())
    }
}

impl GasValidation for CreateTransaction {
    fn validate_gas(&self, _gas_used: u64) -> Result<()> {
        Ok(())
    }
}

impl GasValidation for ScriptTransaction {
    fn validate_gas(&self, gas_used: u64) -> Result<()> {
        if gas_used > *self.tx.script_gas_limit() {
            return Err(error_transaction!(
                Validation,
                "script_gas_limit({}) is lower than the estimated gas_used({})",
                self.tx.script_gas_limit(),
                gas_used
            ));
        }

        Ok(())
    }
}

impl ScriptTransaction {
    pub fn script(&self) -> &Vec<u8> {
        self.tx.script()
    }

    pub fn script_data(&self) -> &Vec<u8> {
        self.tx.script_data()
    }

    pub fn gas_limit(&self) -> u64 {
        *self.tx.script_gas_limit()
    }

    pub fn with_gas_limit(mut self, gas_limit: u64) -> Self {
        self.tx.set_script_gas_limit(gas_limit);
        self
    }
}

#[cfg(test)]
mod test {

    use fuel_tx::policies::Policies;

    use super::*;

    #[test]
    fn append_witnesses_returns_error_when_limit_exceeded() {
        let mut tx = ScriptTransaction {
            tx: FuelTransaction::script(
                0,
                vec![],
                vec![],
                Policies::default(),
                vec![],
                vec![],
                vec![],
            ),
            is_using_predicates: false,
        };

        let witness = vec![0, 1, 2].into();
        let err = tx.append_witness(witness).expect_err("should error");

        let expected_err_str = "transaction validation: Witness limit exceeded. \
                                Consider setting the limit manually with a transaction builder. \
                                The new limit should be: `16`";

        assert_eq!(&err.to_string(), expected_err_str);
    }
}
