use std::{collections::HashSet, fmt::Debug, marker::PhantomData};

use fuel_tx::{Bytes32, ContractId, Output, Receipt};
use fuel_types::bytes::padded_len_usize;
use fuels_accounts::{
    provider::{Provider, TransactionCost},
    Account,
};
use fuels_core::{
    codec::DecoderConfig,
    constants::BASE_ASSET_ID,
    offsets::base_offset_script,
    traits::{Parameterize, Tokenizable},
    types::{
        bech32::Bech32ContractId,
        errors::Result,
        input::Input,
        transaction::{ScriptTransaction, Transaction, TxParameters},
        transaction_builders::ScriptTransactionBuilder,
        unresolved_bytes::UnresolvedBytes,
    },
};
use itertools::chain;

use crate::{
    call_response::FuelCallResponse,
    call_utils::{
        generate_contract_inputs, generate_contract_outputs, new_variable_outputs,
        TxDependencyExtension,
    },
    contract::SettableContract,
    logs::{map_revert_error, LogDecoder},
    receipt_parser::ReceiptParser,
};

#[derive(Debug)]
/// Contains all data relevant to a single script call
pub struct ScriptCall {
    pub script_binary: Vec<u8>,
    pub encoded_args: UnresolvedBytes,
    pub inputs: Vec<Input>,
    pub outputs: Vec<Output>,
    pub external_contracts: Vec<Bech32ContractId>,
    pub variable_outputs: Vec<Output>,
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

    pub fn append_external_contracts(&mut self, contract_id: Bech32ContractId) {
        self.external_contracts.push(contract_id)
    }

    pub fn append_variable_outputs(&mut self, num: u64) {
        self.variable_outputs
            .extend(new_variable_outputs(num as usize));
    }
}

#[derive(Debug)]
#[must_use = "script calls do nothing unless you `call` them"]
/// Helper that handles submitting a script call to a client and formatting the response
pub struct ScriptCallHandler<T: Account, D> {
    pub script_call: ScriptCall,
    pub tx_parameters: TxParameters,
    // Initially `None`, gets set to the right tx id after the transaction is submitted
    cached_tx_id: Option<Bytes32>,
    decoder_config: DecoderConfig,
    pub account: T,
    pub provider: Provider,
    pub datatype: PhantomData<D>,
    pub log_decoder: LogDecoder,
}

impl<T: Account, D> ScriptCallHandler<T, D>
where
    D: Parameterize + Tokenizable + Debug,
{
    pub fn new(
        script_binary: Vec<u8>,
        encoded_args: UnresolvedBytes,
        account: T,
        provider: Provider,
        log_decoder: LogDecoder,
    ) -> Self {
        let script_call = ScriptCall {
            script_binary,
            encoded_args,
            inputs: vec![],
            outputs: vec![],
            external_contracts: vec![],
            variable_outputs: vec![],
        };
        Self {
            script_call,
            tx_parameters: TxParameters::default(),
            cached_tx_id: None,
            account,
            provider,
            datatype: PhantomData,
            log_decoder,
            decoder_config: DecoderConfig::default(),
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

    pub fn with_decoder_config(mut self, decoder_config: DecoderConfig) -> Self {
        self.decoder_config = decoder_config;
        self.log_decoder.set_decoder_config(decoder_config);
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

    pub fn with_contract_ids(mut self, contract_ids: &[Bech32ContractId]) -> Self {
        self.script_call.external_contracts = contract_ids.to_vec();
        self
    }

    pub fn with_contracts(mut self, contracts: &[&dyn SettableContract]) -> Self {
        self.script_call.external_contracts = contracts.iter().map(|c| c.id()).collect();
        for c in contracts {
            self.log_decoder.merge(c.log_decoder());
        }
        self
    }

    /// Compute the script data by calculating the script offset and resolving the encoded arguments
    async fn compute_script_data(&self) -> Result<Vec<u8>> {
        let consensus_parameters = self.provider.consensus_parameters();
        let script_offset = base_offset_script(&consensus_parameters)
            + padded_len_usize(self.script_call.script_binary.len());

        Ok(self.script_call.encoded_args.resolve(script_offset as u64))
    }

    async fn prepare_builder(&self) -> Result<ScriptTransactionBuilder> {
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
            self.script_call.variable_outputs.clone(),
        )
        .collect();

        let tb = ScriptTransactionBuilder::prepare_transfer(inputs, outputs, self.tx_parameters)
            .with_script(self.script_call.script_binary.clone())
            .with_script_data(self.compute_script_data().await?);

        Ok(tb)
    }

    fn calculate_base_asset_sum(&self) -> u64 {
        self.script_call
            .inputs
            .iter()
            .map(|input| match input {
                Input::ResourceSigned { resource, .. }
                | Input::ResourcePredicate { resource, .. }
                    if resource.asset_id() == BASE_ASSET_ID =>
                {
                    resource.amount()
                }
                _ => 0,
            })
            .sum()
    }

    /// Returns the transaction that executes the script call
    pub async fn build_tx(&self) -> Result<ScriptTransaction> {
        let tb = self.prepare_builder().await?;
        let base_amount = self.calculate_base_asset_sum();

        self.account.add_fee_resources(tb, base_amount).await
    }

    /// Call a script on the node. If `simulate == true`, then the call is done in a
    /// read-only manner, using a `dry-run`. The [`FuelCallResponse`] struct contains the `main`'s value
    /// in its `value` field as an actual typed value `D` (if your method returns `bool`,
    /// it will be a bool, works also for structs thanks to the `abigen!()`).
    /// The other field of [`FuelCallResponse`], `receipts`, contains the receipts of the transaction.
    async fn call_or_simulate(&mut self, simulate: bool) -> Result<FuelCallResponse<D>> {
        let tx = self.build_tx().await?;
        self.cached_tx_id = Some(tx.id(self.provider.chain_id()));

        let receipts = if simulate {
            self.provider.checked_dry_run(tx).await?
        } else {
            let tx_id = self.provider.send_transaction(tx).await?;
            self.provider.get_receipts(&tx_id).await?
        };

        self.get_response(receipts)
    }

    /// Call a script on the node, in a state-modifying manner.
    pub async fn call(mut self) -> Result<FuelCallResponse<D>> {
        self.call_or_simulate(false)
            .await
            .map_err(|err| map_revert_error(err, &self.log_decoder))
    }

    pub async fn submit(mut self) -> Result<ScriptCallHandler<T, D>> {
        let tx = self.build_tx().await?;
        self.cached_tx_id = Some(self.provider.send_transaction(tx).await?);

        Ok(self)
    }

    pub async fn response(self) -> Result<FuelCallResponse<D>> {
        let receipts = self
            .account
            .try_provider()?
            .get_receipts(&self.cached_tx_id.expect("Cached tx_id is missing"))
            .await?;
        self.get_response(receipts)
    }

    /// Call a script on the node, in a simulated manner, meaning the state of the
    /// blockchain is *not* modified but simulated.
    /// It is the same as the [`call`] method because the API is more user-friendly this way.
    ///
    /// [`call`]: Self::call
    pub async fn simulate(&mut self) -> Result<FuelCallResponse<D>> {
        self.call_or_simulate(true)
            .await
            .map_err(|err| map_revert_error(err, &self.log_decoder))
    }

    /// Get a scripts's estimated cost
    pub async fn estimate_transaction_cost(
        &self,
        tolerance: Option<f64>,
    ) -> Result<TransactionCost> {
        let tb = self.prepare_builder().await?;
        let tx = self.account.add_fee_resources(tb, 0).await?;

        let transaction_cost = self
            .provider
            .estimate_transaction_cost(tx, tolerance)
            .await?;

        Ok(transaction_cost)
    }

    /// Create a [`FuelCallResponse`] from call receipts
    pub fn get_response(&self, receipts: Vec<Receipt>) -> Result<FuelCallResponse<D>> {
        let token =
            ReceiptParser::new(&receipts, self.decoder_config).parse(None, &D::param_type())?;

        Ok(FuelCallResponse::new(
            D::from_token(token)?,
            receipts,
            self.log_decoder.clone(),
            self.cached_tx_id,
        ))
    }
}

#[async_trait::async_trait]
impl<T, D> TxDependencyExtension for ScriptCallHandler<T, D>
where
    T: Account,
    D: Tokenizable + Parameterize + Debug + Send + Sync,
{
    async fn simulate(&mut self) -> Result<()> {
        self.simulate().await?;

        Ok(())
    }

    fn append_variable_outputs(mut self, num: u64) -> Self {
        self.script_call.append_variable_outputs(num);
        self
    }

    fn append_contract(mut self, contract_id: Bech32ContractId) -> Self {
        self.script_call.append_external_contracts(contract_id);
        self
    }
}
