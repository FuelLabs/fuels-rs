use core::{fmt::Debug, marker::PhantomData};

use fuel_tx::{AssetId, Bytes32, Receipt};
use fuels_accounts::{provider::TransactionCost, Account};
use fuels_core::{
    codec::{ABIEncoder, DecoderConfig, EncoderConfig, LogDecoder},
    traits::{Parameterize, Tokenizable},
    types::{
        bech32::{Bech32Address, Bech32ContractId},
        errors::{error, transaction::Reason, Error, Result},
        input::Input,
        output::Output,
        transaction::{ScriptTransaction, Transaction, TxPolicies},
        transaction_builders::{
            BuildableTransaction, ScriptBuildStrategy, ScriptTransactionBuilder,
            VariableOutputPolicy,
        },
        tx_status::TxStatus,
        Selector, Token,
    },
};

use crate::{
    calls::{
        receipt_parser::ReceiptParser,
        traits::{ContractDependencyConfigurator, ResponseParser, TransactionTuner},
        utils::find_id_of_missing_contract,
        CallParameters, ContractCall, Execution, ScriptCall,
    },
    responses::{CallResponse, SubmitResponse},
};

// Trait implemented by contract instances so that
// they can be passed to the `with_contracts` method
pub trait ContractDependency {
    fn id(&self) -> Bech32ContractId;
    fn log_decoder(&self) -> LogDecoder;
}

#[derive(Debug, Clone)]
#[must_use = "contract calls do nothing unless you `call` them"]
/// Helper that handles submitting a call to a client and formatting the response
pub struct CallHandler<A, C, T> {
    pub account: A,
    pub call: C,
    pub tx_policies: TxPolicies,
    pub log_decoder: LogDecoder,
    pub datatype: PhantomData<T>,
    decoder_config: DecoderConfig,
    // Initially `None`, gets set to the right tx id after the transaction is submitted
    cached_tx_id: Option<Bytes32>,
    variable_output_policy: VariableOutputPolicy,
}

impl<A, C, T> CallHandler<A, C, T> {
    /// Sets the transaction policies for a given transaction.
    /// Note that this is a builder method, i.e. use it as a chain:
    /// ```ignore
    /// let tx_policies = TxPolicies::default().with_gas_price(100);
    /// my_contract_instance.my_method(...).with_tx_policies(tx_policies).call()
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

    /// If this method is not called, the default policy is to not add any variable outputs.
    ///
    /// # Parameters
    /// - `variable_outputs`: The [`VariableOutputPolicy`] to apply for the contract call.
    ///
    /// # Returns
    /// - `Self`: The updated SDK configuration.
    pub fn with_variable_output_policy(mut self, variable_outputs: VariableOutputPolicy) -> Self {
        self.variable_output_policy = variable_outputs;
        self
    }
}

impl<A, C, T> CallHandler<A, C, T>
where
    A: Account,
    C: TransactionTuner,
    T: Tokenizable + Parameterize + Debug,
{
    pub async fn transaction_builder(&self) -> Result<ScriptTransactionBuilder> {
        self.call
            .transaction_builder(self.tx_policies, self.variable_output_policy, &self.account)
            .await
    }

    /// Returns the script that executes the contract call
    pub async fn build_tx(&self) -> Result<ScriptTransaction> {
        self.call
            .build_tx(self.tx_policies, self.variable_output_policy, &self.account)
            .await
    }

    /// Get a call's estimated cost
    pub async fn estimate_transaction_cost(
        &self,
        tolerance: Option<f64>,
        block_horizon: Option<u32>,
    ) -> Result<TransactionCost> {
        let tx = self.build_tx().await?;
        let provider = self.account.try_provider()?;

        let transaction_cost = provider
            .estimate_transaction_cost(tx, tolerance, block_horizon)
            .await?;

        Ok(transaction_cost)
    }
}

impl<A, C, T> CallHandler<A, C, T>
where
    A: Account,
    C: ContractDependencyConfigurator + TransactionTuner + ResponseParser,
    T: Tokenizable + Parameterize + Debug,
{
    /// Sets external contracts as dependencies to this contract's call.
    /// Effectively, this will be used to create [`fuel_tx::Input::Contract`]/[`fuel_tx::Output::Contract`]
    /// pairs and set them into the transaction. Note that this is a builder
    /// method, i.e. use it as a chain:
    ///
    /// ```ignore
    /// my_contract_instance.my_method(...).with_contract_ids(&[another_contract_id]).call()
    /// ```
    ///
    /// [`Input::Contract`]: fuel_tx::Input::Contract
    /// [`Output::Contract`]: fuel_tx::Output::Contract
    pub fn with_contract_ids(mut self, contract_ids: &[Bech32ContractId]) -> Self {
        self.call = self.call.with_external_contracts(contract_ids.to_vec());

        self
    }

    /// Sets external contract instances as dependencies to this contract's call.
    /// Effectively, this will be used to: merge `LogDecoder`s and create
    /// [`fuel_tx::Input::Contract`]/[`fuel_tx::Output::Contract`] pairs and set them into the transaction.
    /// Note that this is a builder method, i.e. use it as a chain:
    ///
    /// ```ignore
    /// my_contract_instance.my_method(...).with_contracts(&[another_contract_instance]).call()
    /// ```
    pub fn with_contracts(mut self, contracts: &[&dyn ContractDependency]) -> Self {
        self.call = self
            .call
            .with_external_contracts(contracts.iter().map(|c| c.id()).collect());
        for c in contracts {
            self.log_decoder.merge(c.log_decoder());
        }

        self
    }

    /// Call a contract's method on the node, in a state-modifying manner.
    pub async fn call(mut self) -> Result<CallResponse<T>> {
        let tx = self.build_tx().await?;
        let provider = self.account.try_provider()?;

        let consensus_parameters = provider.consensus_parameters().await?;
        let chain_id = consensus_parameters.chain_id();
        self.cached_tx_id = Some(tx.id(chain_id));

        let tx_status = provider.send_transaction_and_await_commit(tx).await?;

        let receipts = tx_status.take_receipts_checked(Some(&self.log_decoder))?;

        self.get_response(receipts)
    }

    pub async fn submit(mut self) -> Result<SubmitResponse<A, C, T>> {
        let tx = self.build_tx().await?;
        let provider = self.account.try_provider()?;

        let tx_id = provider.send_transaction(tx.clone()).await?;
        self.cached_tx_id = Some(tx_id);

        Ok(SubmitResponse::<A, C, T>::new(tx_id, self))
    }

    /// Call a contract's method on the node, in a simulated manner, meaning the state of the
    /// blockchain is *not* modified but simulated.
    pub async fn simulate(&mut self, execution: Execution) -> Result<CallResponse<T>> {
        let provider = self.account.try_provider()?;

        let tx_status = if let Execution::StateReadOnly = execution {
            let tx = self
                .transaction_builder()
                .await?
                .with_build_strategy(ScriptBuildStrategy::StateReadOnly)
                .build(provider)
                .await?;

            provider.dry_run_opt(tx, false, Some(0)).await?
        } else {
            let tx = self.build_tx().await?;
            provider.dry_run(tx).await?
        };
        let receipts = tx_status.take_receipts_checked(Some(&self.log_decoder))?;

        self.get_response(receipts)
    }

    /// Create a [`CallResponse`] from call receipts
    pub fn get_response(&self, receipts: Vec<Receipt>) -> Result<CallResponse<T>> {
        let token = self
            .call
            .parse_call(&receipts, self.decoder_config, &T::param_type())?;

        Ok(CallResponse::new(
            T::from_token(token)?,
            receipts,
            self.log_decoder.clone(),
            self.cached_tx_id,
        ))
    }

    /// Create a [`CallResponse`] from `TxStatus`
    pub fn get_response_from(&self, tx_status: TxStatus) -> Result<CallResponse<T>> {
        let receipts = tx_status.take_receipts_checked(Some(&self.log_decoder))?;

        self.get_response(receipts)
    }

    pub async fn determine_missing_contracts(mut self, max_attempts: Option<u64>) -> Result<Self> {
        let attempts = max_attempts.unwrap_or(10);

        for _ in 0..attempts {
            match self.simulate(Execution::Realistic).await {
                Ok(_) => return Ok(self),

                Err(Error::Transaction(Reason::Reverted { ref receipts, .. })) => {
                    if let Some(contract_id) = find_id_of_missing_contract(receipts) {
                        self.call.append_external_contract(contract_id);
                    }
                }

                Err(other_error) => return Err(other_error),
            }
        }

        self.simulate(Execution::Realistic).await.map(|_| self)
    }
}

impl<A, T> CallHandler<A, ContractCall, T>
where
    A: Account,
    T: Tokenizable + Parameterize + Debug,
{
    pub fn new_contract_call(
        contract_id: Bech32ContractId,
        account: A,
        encoded_selector: Selector,
        args: &[Token],
        log_decoder: LogDecoder,
        is_payable: bool,
        encoder_config: EncoderConfig,
    ) -> Self {
        let call = ContractCall {
            contract_id,
            encoded_selector,
            encoded_args: ABIEncoder::new(encoder_config).encode(args),
            call_parameters: CallParameters::default(),
            external_contracts: vec![],
            output_param: T::param_type(),
            is_payable,
            custom_assets: Default::default(),
        };
        CallHandler {
            account,
            call,
            tx_policies: TxPolicies::default(),
            log_decoder,
            datatype: PhantomData,
            decoder_config: DecoderConfig::default(),
            cached_tx_id: None,
            variable_output_policy: VariableOutputPolicy::default(),
        }
    }

    /// Adds a custom `asset_id` with its `amount` and an optional `address` to be used for
    /// generating outputs to this contract's call.
    ///
    /// # Parameters
    /// - `asset_id`: The unique identifier of the asset being added.
    /// - `amount`: The amount of the asset being added.
    /// - `address`: The optional account address that the output amount will be sent to.
    ///              If not provided, the asset will be sent to the users account address.
    ///
    /// Note that this is a builder method, i.e. use it as a chain:
    ///
    /// ```ignore
    /// let asset_id = AssetId::from([3u8; 32]);
    /// let amount = 5000;
    /// my_contract_instance.my_method(...).add_custom_asset(asset_id, amount, None).call()
    /// ```
    pub fn add_custom_asset(
        mut self,
        asset_id: AssetId,
        amount: u64,
        to: Option<Bech32Address>,
    ) -> Self {
        self.call.add_custom_asset(asset_id, amount, to);
        self
    }

    pub fn is_payable(&self) -> bool {
        self.call.is_payable
    }

    /// Sets the call parameters for a given contract call.
    /// Note that this is a builder method, i.e. use it as a chain:
    ///
    /// ```ignore
    /// let params = CallParameters { amount: 1, asset_id: AssetId::zeroed() };
    /// my_contract_instance.my_method(...).call_params(params).call()
    /// ```
    pub fn call_params(mut self, params: CallParameters) -> Result<Self> {
        if !self.is_payable() && params.amount() > 0 {
            return Err(error!(Other, "assets forwarded to non-payable method"));
        }
        self.call.call_parameters = params;

        Ok(self)
    }
}

impl<A, T> CallHandler<A, ScriptCall, T>
where
    A: Account,
    T: Parameterize + Tokenizable + Debug,
{
    pub fn new_script_call(
        script_binary: Vec<u8>,
        encoded_args: Result<Vec<u8>>,
        account: A,
        log_decoder: LogDecoder,
    ) -> Self {
        let call = ScriptCall {
            script_binary,
            encoded_args,
            inputs: vec![],
            outputs: vec![],
            external_contracts: vec![],
        };

        Self {
            account,
            call,
            tx_policies: TxPolicies::default(),
            log_decoder,
            datatype: PhantomData,
            decoder_config: DecoderConfig::default(),
            cached_tx_id: None,
            variable_output_policy: VariableOutputPolicy::default(),
        }
    }

    pub fn with_outputs(mut self, outputs: Vec<Output>) -> Self {
        self.call = self.call.with_outputs(outputs);
        self
    }

    pub fn with_inputs(mut self, inputs: Vec<Input>) -> Self {
        self.call = self.call.with_inputs(inputs);
        self
    }
}

impl<A> CallHandler<A, Vec<ContractCall>, ()>
where
    A: Account,
{
    pub fn new_multi_call(account: A) -> Self {
        Self {
            account,
            call: vec![],
            tx_policies: TxPolicies::default(),
            log_decoder: LogDecoder::new(Default::default()),
            datatype: PhantomData,
            decoder_config: DecoderConfig::default(),
            cached_tx_id: None,
            variable_output_policy: VariableOutputPolicy::default(),
        }
    }

    fn append_external_contract(mut self, contract_id: Bech32ContractId) -> Result<Self> {
        if self.call.is_empty() {
            return Err(error!(
                Other,
                "no calls added. Have you used '.add_calls()'?"
            ));
        }

        self.call
            .iter_mut()
            .take(1)
            .for_each(|call| call.append_external_contract(contract_id.clone()));

        Ok(self)
    }

    /// Adds a contract call to be bundled in the transaction
    /// Note that this is a builder method
    pub fn add_call(
        mut self,
        call_handler: CallHandler<impl Account, ContractCall, impl Tokenizable>,
    ) -> Self {
        self.log_decoder.merge(call_handler.log_decoder);
        self.call.push(call_handler.call);

        self
    }

    /// Call contract methods on the node, in a state-modifying manner.
    pub async fn call<T: Tokenizable + Debug>(mut self) -> Result<CallResponse<T>> {
        let tx = self.build_tx().await?;

        let provider = self.account.try_provider()?;
        let consensus_parameters = provider.consensus_parameters().await?;
        let chain_id = consensus_parameters.chain_id();

        self.cached_tx_id = Some(tx.id(chain_id));

        let tx_status = provider.send_transaction_and_await_commit(tx).await?;

        let receipts = tx_status.take_receipts_checked(Some(&self.log_decoder))?;
        self.get_response(receipts)
    }

    pub async fn submit(mut self) -> Result<SubmitResponse<A, Vec<ContractCall>, ()>> {
        let tx = self.build_tx().await?;
        let provider = self.account.try_provider()?;

        let tx_id = provider.send_transaction(tx).await?;
        self.cached_tx_id = Some(tx_id);

        Ok(SubmitResponse::<A, Vec<ContractCall>, ()>::new(tx_id, self))
    }

    /// Call contract methods on the node, in a simulated manner, meaning the state of the
    /// blockchain is *not* modified but simulated.
    /// It is the same as the [call] method because the API is more user-friendly this way.
    ///
    /// [call]: Self::call
    pub async fn simulate<T: Tokenizable + Debug>(
        &mut self,
        execution: Execution,
    ) -> Result<CallResponse<T>> {
        let provider = self.account.try_provider()?;

        let tx_status = if let Execution::StateReadOnly = execution {
            let tx = self
                .transaction_builder()
                .await?
                .with_build_strategy(ScriptBuildStrategy::StateReadOnly)
                .build(provider)
                .await?;

            provider.dry_run_opt(tx, false, Some(0)).await?
        } else {
            let tx = self.build_tx().await?;
            provider.dry_run(tx).await?
        };
        let receipts = tx_status.take_receipts_checked(Some(&self.log_decoder))?;

        self.get_response(receipts)
    }

    /// Simulates a call without needing to resolve the generic for the return type
    async fn simulate_without_decode(&self) -> Result<()> {
        let provider = self.account.try_provider()?;
        let tx = self.build_tx().await?;

        provider.dry_run(tx).await?.check(None)?;

        Ok(())
    }

    /// Create a [`CallResponse`] from call receipts
    pub fn get_response<T: Tokenizable + Debug>(
        &self,
        receipts: Vec<Receipt>,
    ) -> Result<CallResponse<T>> {
        let mut receipt_parser = ReceiptParser::new(&receipts, self.decoder_config);

        let final_tokens = self
            .call
            .iter()
            .map(|call| receipt_parser.parse_call(&call.contract_id, &call.output_param))
            .collect::<Result<Vec<_>>>()?;

        let tokens_as_tuple = Token::Tuple(final_tokens);
        let response = CallResponse::<T>::new(
            T::from_token(tokens_as_tuple)?,
            receipts,
            self.log_decoder.clone(),
            self.cached_tx_id,
        );

        Ok(response)
    }

    /// Simulates the call and attempts to resolve missing contract outputs.
    /// Forwards the received error if it cannot be fixed.
    pub async fn determine_missing_contracts(mut self, max_attempts: Option<u64>) -> Result<Self> {
        let attempts = max_attempts.unwrap_or(10);

        for _ in 0..attempts {
            match self.simulate_without_decode().await {
                Ok(_) => return Ok(self),

                Err(Error::Transaction(Reason::Reverted { ref receipts, .. })) => {
                    if let Some(contract_id) = find_id_of_missing_contract(receipts) {
                        self = self.append_external_contract(contract_id)?;
                    }
                }

                Err(other_error) => return Err(other_error),
            }
        }

        self.simulate_without_decode().await.map(|_| self)
    }
}
