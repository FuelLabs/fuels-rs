use std::{fmt::Debug, marker::PhantomData};

use fuel_tx::{AssetId, Bytes32, Receipt};
use fuels_accounts::{provider::TransactionCost, Account};
use fuels_core::{
    codec::{ABIEncoder, DecoderConfig, EncoderConfig, LogDecoder},
    traits::{Parameterize, Tokenizable},
    types::{
        bech32::{Bech32Address, Bech32ContractId},
        errors::{error, Result},
        input::Input,
        output::Output,
        transaction::{ScriptTransaction, Transaction, TxPolicies},
        transaction_builders::ScriptTransactionBuilder,
        tx_status::TxStatus,
        Selector, Token,
    },
};

use crate::{
    calls::{
        receipt_parser::ReceiptParser,
        traits::{Buildable, Extendable, Parsable},
        utils::sealed,
        CallParameters, ContractCall, ScriptCall, TxDependencyExtension,
    },
    responses::{CallResponse, SubmitResponse},
};

// Trait implemented by contract instances so that
// they can be passed to the `with_contracts` method
pub trait SettableContract {
    fn id(&self) -> Bech32ContractId;
    fn log_decoder(&self) -> LogDecoder;
}

#[derive(Debug, Clone)]
#[must_use = "contract calls do nothing unless you `call` them"]
/// Helper that handles submitting a call to a client and formatting the response
pub struct CallHandler<T, D, C> {
    pub account: T,
    pub call: C,
    pub tx_policies: TxPolicies,
    pub log_decoder: LogDecoder,
    pub datatype: PhantomData<D>,
    decoder_config: DecoderConfig,
    // Initially `None`, gets set to the right tx id after the transaction is submitted
    cached_tx_id: Option<Bytes32>,
}

impl<T, D, C> CallHandler<T, D, C> {
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
}

impl<T, D, C> CallHandler<T, D, C>
where
    T: Account,
    D: Tokenizable + Parameterize + Debug,
    C: Buildable,
{
    pub async fn transaction_builder(&self) -> Result<ScriptTransactionBuilder> {
        self.call
            .transaction_builder(self.tx_policies, &self.account)
            .await
    }

    /// Returns the script that executes the contract call
    pub async fn build_tx(&self) -> Result<ScriptTransaction> {
        self.call.build_tx(self.tx_policies, &self.account).await
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

impl<T, D, C> CallHandler<T, D, C>
where
    T: Account,
    D: Tokenizable + Parameterize + Debug,
    C: Extendable + Buildable + Parsable,
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
    pub fn with_contracts(mut self, contracts: &[&dyn SettableContract]) -> Self {
        self.call = self
            .call
            .with_external_contracts(contracts.iter().map(|c| c.id()).collect());
        for c in contracts {
            self.log_decoder.merge(c.log_decoder());
        }
        self
    }

    /// Call a contract's method on the node, in a state-modifying manner.
    pub async fn call(mut self) -> Result<CallResponse<D>> {
        self.call_or_simulate(false).await
    }

    pub async fn submit(mut self) -> Result<SubmitResponse<T, D, C>> {
        let tx = self.build_tx().await?;
        let provider = self.account.try_provider()?;

        let tx_id = provider.send_transaction(tx.clone()).await?;
        self.cached_tx_id = Some(tx_id);

        Ok(SubmitResponse::<T, D, C>::new(tx_id, self))
    }

    /// Call a contract's method on the node, in a simulated manner, meaning the state of the
    /// blockchain is *not* modified but simulated.
    pub async fn simulate(&mut self) -> Result<CallResponse<D>> {
        self.call_or_simulate(true).await
    }

    async fn call_or_simulate(&mut self, simulate: bool) -> Result<CallResponse<D>> {
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

    /// Create a [`CallResponse`] from call receipts
    pub fn get_response(&self, receipts: Vec<Receipt>) -> Result<CallResponse<D>> {
        let token = self
            .call
            .parse_call(&receipts, self.decoder_config, &D::param_type())?;

        Ok(CallResponse::new(
            D::from_token(token)?,
            receipts,
            self.log_decoder.clone(),
            self.cached_tx_id,
        ))
    }

    /// Create a [`CallResponse`] from `TxStatus`
    pub fn get_response_from(&self, tx_status: TxStatus) -> Result<CallResponse<D>> {
        let receipts = tx_status.take_receipts_checked(Some(&self.log_decoder))?;

        self.get_response(receipts)
    }
}

impl<T, D> CallHandler<T, D, ContractCall>
where
    T: Account,
    D: Tokenizable + Parameterize + Debug,
{
    pub fn new_contract_call(
        contract_id: Bech32ContractId,
        account: T,
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
            variable_outputs: vec![],
            external_contracts: vec![],
            output_param: D::param_type(),
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

impl<T, D> CallHandler<T, D, ScriptCall>
where
    T: Account,
    D: Parameterize + Tokenizable + Debug,
{
    pub fn new_script_call(
        script_binary: Vec<u8>,
        encoded_args: Result<Vec<u8>>,
        account: T,
        log_decoder: LogDecoder,
    ) -> Self {
        let call = ScriptCall {
            script_binary,
            encoded_args,
            inputs: vec![],
            outputs: vec![],
            external_contracts: vec![],
            variable_outputs: vec![],
        };

        Self {
            account,
            call,
            tx_policies: TxPolicies::default(),
            log_decoder,
            datatype: PhantomData,
            decoder_config: DecoderConfig::default(),
            cached_tx_id: None,
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

impl<T, D, C> sealed::Sealed for CallHandler<T, D, C> {}

#[async_trait::async_trait]
impl<T, D, C> TxDependencyExtension for CallHandler<T, D, C>
where
    T: Account,
    D: Tokenizable + Parameterize + Debug + Send + Sync,
    C: Extendable + Buildable + Parsable + Send + Sync,
{
    async fn simulate(&mut self) -> Result<()> {
        self.simulate().await?;

        Ok(())
    }

    fn append_variable_outputs(mut self, num: u64) -> Self {
        self.call.append_variable_outputs(num);

        self
    }

    fn append_contract(mut self, contract_id: Bech32ContractId) -> Self {
        self.call.append_contract(contract_id);

        self
    }
}

impl<T> CallHandler<T, (), Vec<ContractCall>>
where
    T: Account,
{
    pub fn new_multi_call(account: T) -> Self {
        Self {
            account,
            call: vec![],
            tx_policies: TxPolicies::default(),
            log_decoder: LogDecoder::new(Default::default()),
            datatype: PhantomData,
            decoder_config: DecoderConfig::default(),
            cached_tx_id: None,
        }
    }

    /// Adds a contract call to be bundled in the transaction
    /// Note that this is a builder method
    pub fn add_call(
        mut self,
        call_handler: CallHandler<impl Account, impl Tokenizable, ContractCall>,
    ) -> Self {
        self.log_decoder.merge(call_handler.log_decoder);
        self.call.push(call_handler.call);

        self
    }

    /// Call contract methods on the node, in a state-modifying manner.
    pub async fn call<D: Tokenizable + Debug>(mut self) -> Result<CallResponse<D>> {
        self.call_or_simulate(false).await
    }

    pub async fn submit(mut self) -> Result<SubmitResponse<T, (), Vec<ContractCall>>> {
        let tx = self.build_tx().await?;
        let provider = self.account.try_provider()?;

        let tx_id = provider.send_transaction(tx).await?;
        self.cached_tx_id = Some(tx_id);

        Ok(SubmitResponse::<T, (), Vec<ContractCall>>::new(tx_id, self))
    }

    /// Call contract methods on the node, in a simulated manner, meaning the state of the
    /// blockchain is *not* modified but simulated.
    /// It is the same as the [call] method because the API is more user-friendly this way.
    ///
    /// [call]: Self::call
    pub async fn simulate<D: Tokenizable + Debug>(&mut self) -> Result<CallResponse<D>> {
        self.call_or_simulate(true).await
    }

    async fn call_or_simulate<D: Tokenizable + Debug>(
        &mut self,
        simulate: bool,
    ) -> Result<CallResponse<D>> {
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

    /// Simulates a call without needing to resolve the generic for the return type
    async fn simulate_without_decode(&self) -> Result<()> {
        let provider = self.account.try_provider()?;
        let tx = self.build_tx().await?;

        provider.dry_run(tx).await?.check(None)?;

        Ok(())
    }

    /// Create a [`CallResponse`] from call receipts
    pub fn get_response<D: Tokenizable + Debug>(
        &self,
        receipts: Vec<Receipt>,
    ) -> Result<CallResponse<D>> {
        let mut receipt_parser = ReceiptParser::new(&receipts, self.decoder_config);

        let final_tokens = self
            .call
            .iter()
            .map(|call| receipt_parser.parse_call(&call.contract_id, &call.output_param))
            .collect::<Result<Vec<_>>>()?;

        let tokens_as_tuple = Token::Tuple(final_tokens);
        let response = CallResponse::<D>::new(
            D::from_token(tokens_as_tuple)?,
            receipts,
            self.log_decoder.clone(),
            self.cached_tx_id,
        );

        Ok(response)
    }
}

#[async_trait::async_trait]
impl<T> TxDependencyExtension for CallHandler<T, (), Vec<ContractCall>>
where
    T: Account,
{
    async fn simulate(&mut self) -> Result<()> {
        self.simulate_without_decode().await?;

        Ok(())
    }

    fn append_variable_outputs(mut self, num: u64) -> Self {
        self.call
            .iter_mut()
            .take(1)
            .for_each(|call| call.append_variable_outputs(num));

        self
    }

    fn append_contract(mut self, contract_id: Bech32ContractId) -> Self {
        self.call
            .iter_mut()
            .take(1)
            .for_each(|call| call.append_contract(contract_id.clone()));

        self
    }
}
