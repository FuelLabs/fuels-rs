use std::{collections::HashSet, fmt::Debug, marker::PhantomData};

use fuel_tx::{Bytes32, ContractId, Output, Receipt};
use fuel_types::bytes::padded_len_usize;
use fuels_accounts::{
    provider::{Provider, TransactionCost},
    Account,
};
use fuels_core::{
    codec::{DecoderConfig, LogDecoder},
    offsets::base_offset_script,
    traits::{Parameterize, Tokenizable},
    types::{
        bech32::Bech32ContractId,
        errors::Result,
        input::Input,
        transaction::{ScriptTransaction, Transaction, TxPolicies},
        transaction_builders::{
            BuildableTransaction, ScriptTransactionBuilder, TransactionBuilder,
        },
        tx_status::TxStatus,
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
    receipt_parser::ReceiptParser,
    submit_response::SubmitResponse,
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
    pub tx_policies: TxPolicies,
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
            tx_policies: TxPolicies::default(),
            cached_tx_id: None,
            account,
            provider,
            datatype: PhantomData,
            log_decoder,
            decoder_config: DecoderConfig::default(),
        }
    }

    /// Sets the transaction policies for a given transaction.
    /// Note that this is a builder method, i.e. use it as a chain:
    ///
    /// ```ignore
    /// let tx_policies = TxPolicies::default().with_gas_price(100);
    /// instance.main(...).with_tx_policies(tx_policies).call()
    /// ```
    pub fn with_tx_policies(mut self, tx_policies: TxPolicies) -> Self {
        self.tx_policies = tx_policies;
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
        let script_offset = base_offset_script(consensus_parameters)
            + padded_len_usize(self.script_call.script_binary.len());

        Ok(self.script_call.encoded_args.resolve(script_offset as u64))
    }

    async fn prepare_inputs_outputs(&self) -> Result<(Vec<Input>, Vec<Output>)> {
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

        Ok((inputs, outputs))
    }

    pub async fn transaction_builder(&self) -> Result<ScriptTransactionBuilder> {
        let network_info = self.account.try_provider()?.network_info().await?;
        let (inputs, outputs) = self.prepare_inputs_outputs().await?;

        Ok(ScriptTransactionBuilder::new(network_info)
            .with_tx_policies(self.tx_policies)
            .with_script(self.script_call.script_binary.clone())
            .with_script_data(self.compute_script_data().await?)
            .with_inputs(inputs)
            .with_outputs(outputs))
    }

    /// Returns the transaction that executes the script call
    pub async fn build_tx(&self) -> Result<ScriptTransaction> {
        let mut tb = self.transaction_builder().await?;

        self.account.add_witnessses(&mut tb);
        self.account.adjust_for_fee(&mut tb, 0).await?;

        tb.build(self.account.try_provider()?).await
    }

    /// Call a script on the node. If `simulate == true`, then the call is done in a
    /// read-only manner, using a `dry-run`. The [`FuelCallResponse`] struct contains the `main`'s value
    /// in its `value` field as an actual typed value `D` (if your method returns `bool`,
    /// it will be a bool, works also for structs thanks to the `abigen!()`).
    /// The other field of [`FuelCallResponse`], `receipts`, contains the receipts of the transaction.
    async fn call_or_simulate(&mut self, simulate: bool) -> Result<FuelCallResponse<D>> {
        let tx = self.build_tx().await?;

        self.cached_tx_id = Some(tx.id(self.provider.chain_id()));

        let tx_status = if simulate {
            self.provider.checked_dry_run(tx).await?
        } else {
            let tx_id = self.provider.send_transaction_and_await_commit(tx).await?;
            self.provider.tx_status(&tx_id).await?
        };
        let receipts = tx_status.take_receipts_checked(Some(&self.log_decoder))?;

        self.get_response(receipts)
    }

    /// Call a script on the node, in a state-modifying manner.
    pub async fn call(mut self) -> Result<FuelCallResponse<D>> {
        self.call_or_simulate(false).await
    }

    pub async fn submit(mut self) -> Result<SubmitResponse<T, D>> {
        let tx = self.build_tx().await?;
        let tx_id = self.provider.send_transaction(tx).await?;
        self.cached_tx_id = Some(tx_id);

        Ok(SubmitResponse::new(tx_id, self))
    }

    pub async fn response(self) -> Result<FuelCallResponse<D>> {
        let tx_id = self.cached_tx_id.expect("Cached tx_id is missing");

        let receipts = self
            .provider
            .tx_status(&tx_id)
            .await?
            .take_receipts_checked(Some(&self.log_decoder))?;

        self.get_response(receipts)
    }

    /// Call a script on the node, in a simulated manner, meaning the state of the
    /// blockchain is *not* modified but simulated.
    /// It is the same as the [`call`] method because the API is more user-friendly this way.
    ///
    /// [`call`]: Self::call
    pub async fn simulate(&mut self) -> Result<FuelCallResponse<D>> {
        self.call_or_simulate(true).await
    }

    /// Get a scripts's estimated cost
    pub async fn estimate_transaction_cost(
        &self,
        tolerance: Option<f64>,
    ) -> Result<TransactionCost> {
        let tx = self.build_tx().await?;

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

    /// Create a [`FuelCallResponse`] from `TxStatus`
    pub fn get_response_from(&self, tx_status: TxStatus) -> Result<FuelCallResponse<D>> {
        let receipts = tx_status.take_receipts_checked(Some(&self.log_decoder))?;

        self.get_response(receipts)
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
