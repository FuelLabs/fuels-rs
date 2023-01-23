use std::{collections::HashSet, fmt::Debug, marker::PhantomData};

use fuel_tx::{ContractId, Input, Output, Receipt, Transaction};
use fuel_types::bytes::padded_len_usize;
use fuels_core::{
    abi_encoder::UnresolvedBytes,
    offsets::base_offset,
    parameters::{CallParameters, TxParameters},
};
use fuels_signers::{provider::Provider, WalletUnlocked};
use fuels_types::{
    bech32::Bech32ContractId,
    errors::Error,
    traits::{Parameterize, Tokenizable},
};
use itertools::chain;

use crate::{
    call_response::FuelCallResponse,
    call_utils::{generate_contract_inputs, generate_contract_outputs},
    contract::{get_decoded_output, SettableContract},
    execution_script::ExecutableFuelCall,
    logs::{decode_revert_error, LogDecoder},
};

#[derive(Debug)]
/// Contains all data relevant to a single script call
pub struct ScriptCall {
    pub script_binary: Vec<u8>,
    pub encoded_args: UnresolvedBytes,
    pub inputs: Vec<Input>,
    pub outputs: Vec<Output>,
    pub external_contracts: Vec<Bech32ContractId>,
    // This field is not currently used but it will be in the future.
    pub call_parameters: CallParameters,
}

impl ScriptCall {
    pub fn with_outputs(mut self, outputs: Vec<Output>) -> Self {
        self.outputs = outputs;
        self
    }

    pub fn with_inputs(mut self, inputs: Vec<Input>) -> Self {
        self.inputs = inputs;
        self
    }

    pub fn with_external_contracts(self, external_contracts: Vec<Bech32ContractId>) -> ScriptCall {
        ScriptCall {
            external_contracts,
            ..self
        }
    }
}

#[derive(Debug)]
#[must_use = "script calls do nothing unless you `call` them"]
/// Helper that handles submitting a script call to a client and formatting the response
pub struct ScriptCallHandler<D> {
    pub script_call: ScriptCall,
    pub tx_parameters: TxParameters,
    pub wallet: WalletUnlocked,
    pub provider: Provider,
    pub datatype: PhantomData<D>,
    pub log_decoder: LogDecoder,
}

impl<D> ScriptCallHandler<D>
where
    D: Parameterize + Tokenizable + Debug,
{
    pub fn new(
        script_binary: Vec<u8>,
        encoded_args: UnresolvedBytes,
        wallet: WalletUnlocked,
        provider: Provider,
        log_decoder: LogDecoder,
    ) -> Self {
        let script_call = ScriptCall {
            script_binary,
            encoded_args,
            inputs: vec![],
            outputs: vec![],
            external_contracts: vec![],
            call_parameters: Default::default(),
        };
        Self {
            script_call,
            tx_parameters: TxParameters::default(),
            wallet,
            provider,
            datatype: PhantomData,
            log_decoder,
        }
    }

    /// Sets the transaction parameters for a given transaction.
    /// Note that this is a builder method, i.e. use it as a chain:
    ///
    /// ```ignore
    /// let params = TxParameters { gas_price: 100, gas_limit: 1000000 };
    /// instance.main(...).tx_params(params).call()
    /// ```
    pub fn tx_params(mut self, params: TxParameters) -> Self {
        self.tx_parameters = params;
        self
    }

    pub fn with_outputs(mut self, outputs: Vec<Output>) -> Self {
        self.script_call = self.script_call.with_outputs(outputs);
        self
    }

    pub fn with_inputs(mut self, inputs: Vec<Input>) -> Self {
        self.script_call = self.script_call.with_inputs(inputs);
        self
    }

    pub fn set_contract_ids(mut self, contract_ids: &[Bech32ContractId]) -> Self {
        self.script_call.external_contracts = contract_ids.to_vec();
        self
    }

    pub fn set_contracts(mut self, contracts: &[&dyn SettableContract]) -> Self {
        self.script_call.external_contracts = contracts.iter().map(|c| c.id()).collect();
        for c in contracts {
            self.log_decoder.merge(c.log_decoder());
        }
        self
    }

    /// Compute the script data by calculating the script offset and resolving the encoded arguments
    async fn compute_script_data(&self) -> Result<Vec<u8>, Error> {
        let consensus_parameters = self.provider.consensus_parameters().await?;
        let script_offset = base_offset(&consensus_parameters)
            + padded_len_usize(self.script_call.script_binary.len());

        Ok(self.script_call.encoded_args.resolve(script_offset as u64))
    }

    /// Call a script on the node. If `simulate == true`, then the call is done in a
    /// read-only manner, using a `dry-run`. The [`FuelCallResponse`] struct contains the `main`'s value
    /// in its `value` field as an actual typed value `D` (if your method returns `bool`,
    /// it will be a bool, works also for structs thanks to the `abigen!()`).
    /// The other field of [`FuelCallResponse`], `receipts`, contains the receipts of the transaction.
    async fn call_or_simulate(&self, simulate: bool) -> Result<FuelCallResponse<D>, Error> {
        let contract_ids: HashSet<ContractId> = self
            .script_call
            .external_contracts
            .iter()
            .map(|bech32| bech32.into())
            .collect();
        let num_of_contracts = contract_ids.len();

        let inputs = chain!(
            generate_contract_inputs(contract_ids),
            self.script_call.inputs.clone(),
        )
        .collect();

        // Note the contract_outputs need to come first since the
        // contract_inputs are referencing them via `output_index`. The node
        // will, upon receiving our request, use `output_index` to index the
        // `inputs` array we've sent over.
        let outputs = chain!(
            generate_contract_outputs(num_of_contracts),
            self.script_call.outputs.clone(),
        )
        .collect();

        let mut tx = Transaction::script(
            self.tx_parameters.gas_price,
            self.tx_parameters.gas_limit,
            self.tx_parameters.maturity,
            self.script_call.script_binary.clone(),
            self.compute_script_data().await?,
            inputs,
            outputs,
            vec![vec![0, 0].into()], //TODO:(iqdecay): figure out how to have the right witnesses
        );
        self.wallet.add_fee_resources(&mut tx, 0, 0).await?;

        let tx_execution = ExecutableFuelCall { tx };

        let receipts = if simulate {
            tx_execution.simulate(&self.provider).await?
        } else {
            tx_execution.execute(&self.provider).await?
        };

        self.get_response(receipts)
    }

    /// Call a script on the node, in a state-modifying manner.
    pub async fn call(self) -> Result<FuelCallResponse<D>, Error> {
        Self::call_or_simulate(&self, false)
            .await
            .map_err(|err| decode_revert_error(err, &self.log_decoder))
    }

    /// Call a script on the node, in a simulated manner, meaning the state of the
    /// blockchain is *not* modified but simulated.
    /// It is the same as the [`call`] method because the API is more user-friendly this way.
    ///
    /// [`call`]: Self::call
    pub async fn simulate(self) -> Result<FuelCallResponse<D>, Error> {
        Self::call_or_simulate(&self, true)
            .await
            .map_err(|err| decode_revert_error(err, &self.log_decoder))
    }

    /// Create a [`FuelCallResponse`] from call receipts
    pub fn get_response(&self, receipts: Vec<Receipt>) -> Result<FuelCallResponse<D>, Error> {
        let token = get_decoded_output(&receipts, None, &D::param_type())?;
        Ok(FuelCallResponse::new(
            D::from_token(token)?,
            receipts,
            self.log_decoder.clone(),
        ))
    }
}
