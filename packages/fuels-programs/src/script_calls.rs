use std::{collections::HashSet, fmt::Debug, marker::PhantomData};

use fuel_tx::{Bytes32, ContractId, Output, Receipt};
use fuels_accounts::{provider::TransactionCost, Account};
use fuels_core::{
    codec::{DecoderConfig, LogDecoder},
    error,
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
    },
};
use itertools::chain;

use crate::{
    call_response::FuelCallResponse,
    call_utils::{
        generate_contract_inputs, generate_contract_outputs, new_variable_outputs, sealed,
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
    pub encoded_args: Result<Vec<u8>>,
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
    pub datatype: PhantomData<D>,
    pub log_decoder: LogDecoder,
}

impl<T: Account, D> ScriptCallHandler<T, D>
where
    D: Parameterize + Tokenizable + Debug,
{
    pub fn new(
        script_binary: Vec<u8>,
        encoded_args: Result<Vec<u8>>,
        account: T,
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

    fn compute_script_data(&self) -> Result<Vec<u8>> {
        self.script_call
            .encoded_args
            .as_ref()
            .map(|b| b.to_owned())
            .map_err(|e| error!(Codec, "cannot encode script call arguments: {e}"))
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
        let (inputs, outputs) = self.prepare_inputs_outputs().await?;

        Ok(ScriptTransactionBuilder::default()
            .with_tx_policies(self.tx_policies)
            .with_script(self.script_call.script_binary.clone())
            .with_script_data(self.compute_script_data()?)
            .with_inputs(inputs)
            .with_outputs(outputs))
    }

    /// Returns the transaction that executes the script call
    pub async fn build_tx(&self) -> Result<ScriptTransaction> {
        let mut tb = self.transaction_builder().await?;

        self.account.add_witnesses(&mut tb)?;
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
        let provider = self.account.try_provider()?;

        self.cached_tx_id = Some(tx.id(provider.chain_id()));

        let tx_status = if simulate {
            provider.dry_run(tx).await?
        } else {
            provider.send_transaction_and_await_commit(tx).await?
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
        let provider = self.account.try_provider()?;

        let tx_id = provider.send_transaction(tx).await?;
        self.cached_tx_id = Some(tx_id);

        Ok(SubmitResponse::new(tx_id, self))
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
        block_horizon: Option<u32>,
    ) -> Result<TransactionCost> {
        let tx = self.build_tx().await?;

        let transaction_cost = self
            .account
            .try_provider()?
            .estimate_transaction_cost(tx, tolerance, block_horizon)
            .await?;

        Ok(transaction_cost)
    }

    /// Create a [`FuelCallResponse`] from call receipts
    pub fn get_response(&self, receipts: Vec<Receipt>) -> Result<FuelCallResponse<D>> {
        let token =
            ReceiptParser::new(&receipts, self.decoder_config).parse_script(&D::param_type())?;

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

impl<T: Account, D> sealed::Sealed for ScriptCallHandler<T, D> {}

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
