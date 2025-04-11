use std::{collections::HashSet, fmt::Debug};

use fuel_tx::{ContractId, Output};
use fuels_core::types::{
    bech32::Bech32ContractId,
    errors::{Result, error},
    input::Input,
};
use itertools::chain;

use crate::calls::utils::{generate_contract_inputs, generate_contract_outputs, sealed};

#[derive(Debug, Clone)]
/// Contains all data relevant to a single script call
pub struct ScriptCall {
    pub script_binary: Vec<u8>,
    pub encoded_args: Result<Vec<u8>>,
    pub inputs: Vec<Input>,
    pub outputs: Vec<Output>,
    pub external_contracts: Vec<Bech32ContractId>,
}

impl ScriptCall {
    /// Add custom outputs to the `ScriptCall`.
    pub fn with_outputs(mut self, outputs: Vec<Output>) -> Self {
        self.outputs = outputs;
        self
    }

    /// Add custom inputs to the `ScriptCall`.
    pub fn with_inputs(mut self, inputs: Vec<Input>) -> Self {
        self.inputs = inputs;
        self
    }

    pub(crate) fn prepare_inputs_outputs(&self) -> Result<(Vec<Input>, Vec<Output>)> {
        let contract_ids: HashSet<ContractId> = self
            .external_contracts
            .iter()
            .map(|bech32| bech32.into())
            .collect();
        let num_of_contracts = contract_ids.len();

        let inputs = chain!(
            self.inputs.clone(),
            generate_contract_inputs(contract_ids, self.outputs.len())
        )
        .collect();

        // Note the contract_outputs are placed after the custom outputs and
        // the contract_inputs are referencing them via `output_index`. The
        // node will, upon receiving our request, use `output_index` to index
        // the `inputs` array we've sent over.
        let outputs = chain!(
            self.outputs.clone(),
            generate_contract_outputs(num_of_contracts, self.inputs.len()),
        )
        .collect();

        Ok((inputs, outputs))
    }

    pub(crate) fn compute_script_data(&self) -> Result<Vec<u8>> {
        self.encoded_args
            .as_ref()
            .map(|b| b.to_owned())
            .map_err(|e| error!(Codec, "cannot encode script call arguments: {e}"))
    }
}

impl sealed::Sealed for ScriptCall {}
