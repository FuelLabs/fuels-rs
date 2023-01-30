use std::fmt::Debug;

use fuel_tx::field::{
    GasLimit, GasPrice, Inputs, Maturity, Outputs, Script as ScriptField, ScriptData, Witnesses,
};
use fuel_tx::{
    Bytes32, Chargeable, Checkable, ConsensusParameters, Input, Output, Script, Transaction,
    TransactionFee, UniqueIdentifier, Witness,
};

use crate::errors::Error;

#[derive(Debug, Clone)]
pub struct ScriptTransaction {
    tx: Script,
}

impl From<Script> for ScriptTransaction {
    fn from(tx: Script) -> Self {
        ScriptTransaction { tx }
    }
}

impl From<ScriptTransaction> for Script {
    fn from(script_tx: ScriptTransaction) -> Self {
        script_tx.tx
    }
}

impl From<ScriptTransaction> for Transaction {
    fn from(script_tx: ScriptTransaction) -> Self {
        script_tx.tx.into()
    }
}

impl ScriptTransaction {
    pub fn new(tx: Script) -> Self {
        Self { tx }
    }

    pub fn fee_checked_from_tx(&self, params: &ConsensusParameters) -> Option<TransactionFee> {
        TransactionFee::checked_from_tx(params, &self.tx)
    }

    pub fn check_without_signatures(
        &self,
        block_height: u64,
        parameters: &ConsensusParameters,
    ) -> Result<(), Error> {
        Ok(self.tx.check_without_signatures(block_height, parameters)?)
    }

    pub fn id(&self) -> Bytes32 {
        self.tx.id()
    }

    pub fn gas_price(&self) -> u64 {
        *self.tx.gas_price()
    }

    pub fn gas_limit(&self) -> u64 {
        *self.tx.gas_limit()
    }

    pub fn gas_price_mut(&mut self) -> &mut u64 {
        self.tx.gas_price_mut()
    }

    pub fn gas_limit_mut(&mut self) -> &mut u64 {
        self.tx.gas_limit_mut()
    }

    pub fn metered_bytes_size(&self) -> usize {
        self.tx.metered_bytes_size()
    }

    pub fn maturity(&self) -> u64 {
        *self.tx.maturity()
    }

    pub fn script(&self) -> &Vec<u8> {
        self.tx.script()
    }

    pub fn script_data(&self) -> &Vec<u8> {
        self.tx.script_data()
    }

    pub fn inputs(&self) -> &Vec<Input> {
        self.tx.inputs()
    }

    pub fn outputs(&self) -> &Vec<Output> {
        self.tx.outputs()
    }

    pub fn inputs_mut(&mut self) -> &mut Vec<Input> {
        self.tx.inputs_mut()
    }

    pub fn outputs_mut(&mut self) -> &mut Vec<Output> {
        self.tx.outputs_mut()
    }

    pub fn witnesses(&self) -> &Vec<Witness> {
        self.tx.witnesses()
    }

    pub fn witnesses_mut(&mut self) -> &mut Vec<Witness> {
        self.tx.witnesses_mut()
    }
}
