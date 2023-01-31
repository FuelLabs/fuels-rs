use std::{collections::HashMap, fmt::Debug, marker::PhantomData, panic};

use fuel_tx::{AssetId, Output, Receipt, Transaction};

use fuels_core::{
    abi_encoder::UnresolvedBytes,
    offsets::call_script_data_offset,
    parameters::{CallParameters, TxParameters},
};
use fuels_signers::{
    provider::{Provider, TransactionCost},
    Signer, WalletUnlocked,
};
use fuels_types::{
    bech32::{Bech32Address, Bech32ContractId},
    core::{Selector, Token},
    errors::{Error, Result},
    param_types::ParamType,
    traits::Tokenizable,
};

use crate::calls::contract_call_utils::{
    build_script_data_from_contract_calls, calculate_required_asset_amounts, get_instructions,
    get_single_call_instructions, get_transaction_inputs_outputs, CallOpcodeParamsOffset,
};
use crate::{
    calls::call::{ProgramCall, SettableContract},
    calls::call_response::FuelCallResponse,
    calls::call_utils::get_decoded_output,
    execution_script::ExecutableFuelCall,
    logs::{decode_revert_error, LogDecoder},
};

/// How many times to attempt to resolve missing tx dependencies.
pub const DEFAULT_TX_DEP_ESTIMATION_ATTEMPTS: u64 = 10;

#[derive(Debug)]
/// Contains all data relevant to a single contract call
pub struct ContractCall {
    pub contract_id: Bech32ContractId,
    pub encoded_args: UnresolvedBytes,
    pub encoded_selector: Selector,
    pub call_parameters: CallParameters,
    pub compute_custom_input_offset: bool,
    pub variable_outputs: Vec<Output>,
    pub message_outputs: Vec<Output>,
    pub external_contracts: Vec<Bech32ContractId>,
    pub output_param: ParamType,
    pub custom_assets: HashMap<(AssetId, Option<Bech32Address>), u64>,
}

impl ContractCall {
    pub fn with_contract_id(self, contract_id: Bech32ContractId) -> Self {
        ContractCall {
            contract_id,
            ..self
        }
    }

    pub fn add_custom_asset(&mut self, asset_id: AssetId, amount: u64, to: Option<Bech32Address>) {
        *self.custom_assets.entry((asset_id, to)).or_default() += amount;
    }
}

impl ExecutableFuelCall {
    /// Creates a [`ExecutableFuelCall`] from contract calls. The internal [Transaction] is
    /// initialized with the actual script instructions, script data needed to perform the call and
    /// transaction inputs/outputs consisting of assets and contracts.
    pub async fn from_contract_calls(
        calls: &[ContractCall],
        tx_parameters: &TxParameters,
        wallet: &WalletUnlocked,
    ) -> Result<Self> {
        let consensus_parameters = wallet.get_provider()?.consensus_parameters().await?;

        // Calculate instructions length for call instructions
        // Use placeholder for call param offsets, we only care about the length
        let calls_instructions_len =
            get_single_call_instructions(&CallOpcodeParamsOffset::default()).len() * calls.len();

        let data_offset = call_script_data_offset(&consensus_parameters, calls_instructions_len);

        let (script_data, call_param_offsets) =
            build_script_data_from_contract_calls(calls, data_offset, tx_parameters.gas_limit);

        let script = get_instructions(calls, call_param_offsets);

        let required_asset_amounts = calculate_required_asset_amounts(calls);
        let mut spendable_resources = vec![];

        // Find the spendable resources required for those calls
        for (asset_id, amount) in &required_asset_amounts {
            let resources = wallet.get_spendable_resources(*asset_id, *amount).await?;
            spendable_resources.extend(resources);
        }

        let (inputs, outputs) =
            get_transaction_inputs_outputs(calls, wallet.address(), spendable_resources);

        let mut tx = Transaction::script(
            tx_parameters.gas_price,
            tx_parameters.gas_limit,
            tx_parameters.maturity,
            script,
            script_data,
            inputs,
            outputs,
            vec![],
        );

        let base_asset_amount = required_asset_amounts
            .iter()
            .find(|(asset_id, _)| *asset_id == AssetId::default());
        match base_asset_amount {
            Some((_, base_amount)) => wallet.add_fee_resources(&mut tx, *base_amount, 0).await?,
            None => wallet.add_fee_resources(&mut tx, 0, 0).await?,
        }
        wallet.sign_transaction(&mut tx).await.unwrap();

        Ok(ExecutableFuelCall::new(tx))
    }
}

#[derive(Debug)]
#[must_use = "contract calls do nothing unless you `call` them"]
/// Helper that handles submitting a call to a client and formatting the response
pub struct ContractCallHandler<D> {
    pub contract_call: ContractCall,
    pub tx_parameters: TxParameters,
    pub wallet: WalletUnlocked,
    pub provider: Provider,
    pub datatype: PhantomData<D>,
    pub log_decoder: LogDecoder,
}

impl<D> ContractCallHandler<D>
where
    D: Tokenizable + Debug,
{
    /// Sets external contracts as dependencies to this contract's call.
    /// Effectively, this will be used to create [`Input::Contract`]/[`Output::Contract`]
    /// pairs and set them into the transaction. Note that this is a builder
    /// method, i.e. use it as a chain:
    ///
    /// ```ignore
    /// my_contract_instance.my_method(...).set_contract_ids(&[another_contract_id]).call()
    /// ```
    ///
    /// [`Input::Contract`]: fuel_tx::Input::Contract
    /// [`Output::Contract`]: fuel_tx::Output::Contract
    pub fn set_contract_ids(mut self, contract_ids: &[Bech32ContractId]) -> Self {
        self.contract_call.external_contracts = contract_ids.to_vec();
        self
    }

    /// Sets external contract instances as dependencies to this contract's call.
    /// Effectively, this will be used to: merge `LogDecoder`s and create
    /// [`Input::Contract`]/[`Output::Contract`] pairs and set them into the transaction.
    /// Note that this is a builder method, i.e. use it as a chain:
    ///
    /// ```ignore
    /// my_contract_instance.my_method(...).set_contracts(&[another_contract_instance]).call()
    /// ```
    /// [`Input::Contract`]: fuel_tx::Input::Contract
    /// [`Output::Contract`]: fuel_tx::Output::Contract
    pub fn set_contracts(mut self, contracts: &[&dyn SettableContract]) -> Self {
        self.contract_call.external_contracts = contracts.iter().map(|c| c.id()).collect();
        for c in contracts {
            self.log_decoder.merge(c.log_decoder());
        }
        self
    }

    /// Adds a custom `asset_id` with its `amount` and an optional `address` to be used for
    /// generating outputs to this contract's call.
    ///
    /// # Parameters
    /// - `asset_id`: The unique identifier of the asset being added.
    /// - `amount`: The amount of the asset being added.
    /// - `address`: The optional wallet address that the output amount will be sent to. If not provided, the asset will be sent to the users wallet address.
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
        self.contract_call.add_custom_asset(asset_id, amount, to);
        self
    }

    /// Appends additional external contracts as dependencies to this contract's
    /// call. Effectively, this will be used to create additional
    /// [`Input::Contract`]/[`Output::Contract`]
    /// pairs and set them into the transaction. Note that this is a builder
    /// method, i.e. use it as a chain:
    ///
    /// ```ignore
    /// my_contract_instance.my_method(...).append_contracts(additional_contract_id).call()
    /// ```
    ///
    /// [`Input::Contract`]: fuel_tx::Input::Contract
    /// [`Output::Contract`]: fuel_tx::Output::Contract
    pub fn append_contract(mut self, contract_id: Bech32ContractId) -> Self {
        self.contract_call.append_external_contracts(contract_id);
        self
    }

    /// Sets the transaction parameters for a given transaction.
    /// Note that this is a builder method, i.e. use it as a chain:

    /// ```ignore
    /// let params = TxParameters { gas_price: 100, gas_limit: 1000000 };
    /// my_contract_instance.my_method(...).tx_params(params).call()
    /// ```
    pub fn tx_params(mut self, params: TxParameters) -> Self {
        self.tx_parameters = params;
        self
    }

    /// Sets the call parameters for a given contract call.
    /// Note that this is a builder method, i.e. use it as a chain:
    ///
    /// ```ignore
    /// let params = CallParameters { amount: 1, asset_id: BASE_ASSET_ID };
    /// my_contract_instance.my_method(...).call_params(params).call()
    /// ```
    pub fn call_params(mut self, params: CallParameters) -> Self {
        self.contract_call.call_parameters = params;
        self
    }

    /// Appends `num` [`Output::Variable`]s to the transaction.
    /// Note that this is a builder method, i.e. use it as a chain:
    ///
    /// ```ignore
    /// my_contract_instance.my_method(...).add_variable_outputs(num).call()
    /// ```
    ///
    /// [`Output::Variable`]: fuel_tx::Output::Variable
    pub fn append_variable_outputs(mut self, num: u64) -> Self {
        self.contract_call.append_variable_outputs(num);
        self
    }

    /// Appends `num` [`Output::Message`]s to the transaction.
    /// Note that this is a builder method, i.e. use it as a chain:
    ///
    /// ```ignore
    /// my_contract_instance.my_method(...).add_message_outputs(num).call()
    /// ```
    ///
    /// [`Output::Message`]: fuel_tx::Output::Message
    pub fn append_message_outputs(mut self, num: u64) -> Self {
        self.contract_call.append_message_outputs(num);
        self
    }

    /// Call a contract's method on the node. If `simulate == true`, then the call is done in a
    /// read-only manner, using a `dry-run`. The [`FuelCallResponse`] struct contains the method's
    /// value in its `value` field as an actual typed value `D` (if your method returns `bool`,
    /// it will be a bool, works also for structs thanks to the `abigen!()`).
    /// The other field of [`FuelCallResponse`], `receipts`, contains the receipts of the transaction.
    async fn call_or_simulate(&self, simulate: bool) -> Result<FuelCallResponse<D>> {
        let script = self.get_executable_call().await?;

        let receipts = if simulate {
            script.simulate(&self.provider).await?
        } else {
            script.execute(&self.provider).await?
        };

        self.get_response(receipts)
    }

    /// Returns the script that executes the contract call
    pub async fn get_executable_call(&self) -> Result<ExecutableFuelCall> {
        ExecutableFuelCall::from_contract_calls(
            std::slice::from_ref(&self.contract_call),
            &self.tx_parameters,
            &self.wallet,
        )
        .await
    }

    /// Call a contract's method on the node, in a state-modifying manner.
    pub async fn call(self) -> Result<FuelCallResponse<D>> {
        Self::call_or_simulate(&self, false)
            .await
            .map_err(|err| decode_revert_error(err, &self.log_decoder))
    }

    /// Call a contract's method on the node, in a simulated manner, meaning the state of the
    /// blockchain is *not* modified but simulated.
    /// It is the same as the [`call`] method because the API is more user-friendly this way.
    ///
    /// [`call`]: Self::call
    pub async fn simulate(self) -> Result<FuelCallResponse<D>> {
        Self::call_or_simulate(&self, true)
            .await
            .map_err(|err| decode_revert_error(err, &self.log_decoder))
    }

    /// Simulates a call without needing to resolve the generic for the return type
    async fn simulate_without_decode(&self) -> Result<()> {
        let script = self.get_executable_call().await?;
        let provider = self.wallet.get_provider()?;

        script.simulate(provider).await?;

        Ok(())
    }

    /// Simulates the call and attempts to resolve missing tx dependencies.
    /// Forwards the received error if it cannot be fixed.
    pub async fn estimate_tx_dependencies(mut self, max_attempts: Option<u64>) -> Result<Self> {
        let attempts = max_attempts.unwrap_or(DEFAULT_TX_DEP_ESTIMATION_ATTEMPTS);

        for _ in 0..attempts {
            let result = self.simulate_without_decode().await;

            match result {
                Err(Error::RevertTransactionError(_, receipts))
                    if ContractCall::is_missing_output_variables(&receipts) =>
                {
                    self = self.append_variable_outputs(1);
                }

                Err(Error::RevertTransactionError(_, ref receipts)) => {
                    if let Some(receipt) = ContractCall::find_contract_not_in_inputs(receipts) {
                        let contract_id = Bech32ContractId::from(*receipt.contract_id().unwrap());
                        self = self.append_contract(contract_id);
                    } else {
                        return Err(result.expect_err("Couldn't estimate tx dependencies because we couldn't find the missing contract input"));
                    }
                }

                Err(e) => return Err(e),
                _ => return Ok(self),
            }
        }

        // confirm if successful or propagate error
        match self.call_or_simulate(true).await {
            Ok(_) => Ok(self),
            Err(e) => Err(e),
        }
    }

    /// Get a contract's estimated cost
    pub async fn estimate_transaction_cost(
        &self,
        tolerance: Option<f64>,
    ) -> Result<TransactionCost> {
        let script = self.get_executable_call().await?;

        let transaction_cost = self
            .provider
            .estimate_transaction_cost(&script.tx, tolerance)
            .await?;

        Ok(transaction_cost)
    }

    /// Create a [`FuelCallResponse`] from call receipts
    pub fn get_response(&self, receipts: Vec<Receipt>) -> Result<FuelCallResponse<D>> {
        let token = get_decoded_output(
            &receipts,
            Some(&self.contract_call.contract_id),
            &self.contract_call.output_param,
        )?;
        Ok(FuelCallResponse::new(
            D::from_token(token)?,
            receipts,
            self.log_decoder.clone(),
        ))
    }
}

#[derive(Debug)]
#[must_use = "contract calls do nothing unless you `call` them"]
/// Helper that handles bundling multiple calls into a single transaction
pub struct MultiContractCallHandler {
    pub contract_calls: Vec<ContractCall>,
    pub log_decoder: LogDecoder,
    pub tx_parameters: TxParameters,
    pub wallet: WalletUnlocked,
}

impl MultiContractCallHandler {
    pub fn new(wallet: WalletUnlocked) -> Self {
        Self {
            contract_calls: vec![],
            tx_parameters: TxParameters::default(),
            wallet,
            log_decoder: LogDecoder {
                type_lookup: HashMap::new(),
            },
        }
    }

    /// Adds a contract call to be bundled in the transaction
    /// Note that this is a builder method
    pub fn add_call<D: Tokenizable>(&mut self, call_handler: ContractCallHandler<D>) -> &mut Self {
        self.log_decoder.merge(call_handler.log_decoder);
        self.contract_calls.push(call_handler.contract_call);
        self
    }

    /// Sets the transaction parameters for a given transaction.
    /// Note that this is a builder method
    pub fn tx_params(&mut self, params: TxParameters) -> &mut Self {
        self.tx_parameters = params;
        self
    }

    /// Returns the script that executes the contract calls
    pub async fn get_executable_call(&self) -> Result<ExecutableFuelCall> {
        if self.contract_calls.is_empty() {
            panic!("No calls added. Have you used '.add_calls()'?");
        }

        ExecutableFuelCall::from_contract_calls(
            &self.contract_calls,
            &self.tx_parameters,
            &self.wallet,
        )
        .await
    }

    /// Call contract methods on the node, in a state-modifying manner.
    pub async fn call<D: Tokenizable + Debug>(&self) -> Result<FuelCallResponse<D>> {
        Self::call_or_simulate(self, false)
            .await
            .map_err(|err| decode_revert_error(err, &self.log_decoder))
    }

    /// Call contract methods on the node, in a simulated manner, meaning the state of the
    /// blockchain is *not* modified but simulated.
    /// It is the same as the [call] method because the API is more user-friendly this way.
    ///
    /// [call]: Self::call
    pub async fn simulate<D: Tokenizable + Debug>(&self) -> Result<FuelCallResponse<D>> {
        Self::call_or_simulate(self, true)
            .await
            .map_err(|err| decode_revert_error(err, &self.log_decoder))
    }

    async fn call_or_simulate<D: Tokenizable + Debug>(
        &self,
        simulate: bool,
    ) -> Result<FuelCallResponse<D>> {
        let script = self.get_executable_call().await?;

        let provider = self.wallet.get_provider()?;

        let receipts = if simulate {
            script.simulate(provider).await?
        } else {
            script.execute(provider).await?
        };

        self.get_response(receipts)
    }

    /// Simulates a call without needing to resolve the generic for the return type
    async fn simulate_without_decode(&self) -> Result<()> {
        let script = self.get_executable_call().await?;
        let provider = self.wallet.get_provider()?;

        script.simulate(provider).await?;

        Ok(())
    }

    /// Simulates the call and attempts to resolve missing tx dependencies.
    /// Forwards the received error if it cannot be fixed.
    pub async fn estimate_tx_dependencies(mut self, max_attempts: Option<u64>) -> Result<Self> {
        let attempts = max_attempts.unwrap_or(DEFAULT_TX_DEP_ESTIMATION_ATTEMPTS);

        for _ in 0..attempts {
            let result = self.simulate_without_decode().await;

            match result {
                Err(Error::RevertTransactionError(_, receipts))
                    if ContractCall::is_missing_output_variables(&receipts) =>
                {
                    self.contract_calls
                        .iter_mut()
                        .take(1)
                        .for_each(|call| call.append_variable_outputs(1));
                }

                Err(Error::RevertTransactionError(_, ref receipts)) => {
                    if let Some(receipt) = ContractCall::find_contract_not_in_inputs(receipts) {
                        let contract_id = Bech32ContractId::from(*receipt.contract_id().unwrap());
                        self.contract_calls
                            .iter_mut()
                            .take(1)
                            .for_each(|call| call.append_external_contracts(contract_id.clone()));
                    } else {
                        return Err(result.expect_err("Couldn't estimate tx dependencies because we couldn't find the missing contract input"));
                    }
                }

                Err(e) => return Err(e),
                _ => return Ok(self),
            }
        }

        Ok(self)
    }

    /// Get a contract's estimated cost
    pub async fn estimate_transaction_cost(
        &self,
        tolerance: Option<f64>,
    ) -> Result<TransactionCost> {
        let script = self.get_executable_call().await?;

        let transaction_cost = self
            .wallet
            .get_provider()?
            .estimate_transaction_cost(&script.tx, tolerance)
            .await?;

        Ok(transaction_cost)
    }

    /// Create a [`FuelCallResponse`] from call receipts
    pub fn get_response<D: Tokenizable + Debug>(
        &self,
        receipts: Vec<Receipt>,
    ) -> Result<FuelCallResponse<D>> {
        let mut final_tokens = vec![];

        for call in self.contract_calls.iter() {
            let decoded =
                get_decoded_output(&receipts, Some(&call.contract_id), &call.output_param)?;

            final_tokens.push(decoded.clone());
        }

        let tokens_as_tuple = Token::Tuple(final_tokens);
        let response = FuelCallResponse::<D>::new(
            D::from_token(tokens_as_tuple)?,
            receipts,
            self.log_decoder.clone(),
        );

        Ok(response)
    }
}
