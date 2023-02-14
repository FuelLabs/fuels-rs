use std::{collections::HashMap, fmt::Debug, marker::PhantomData, panic};

use fuel_abi_types::error_codes::FAILED_TRANSFER_TO_ADDRESS_SIGNAL;
use fuel_tx::{Address, AssetId, Output, Receipt};
use fuel_vm::fuel_asm::PanicReason;

use fuels_core::abi_encoder::UnresolvedBytes;
use fuels_signers::{
    provider::{Provider, TransactionCost}, WalletUnlocked,
};
use fuels_types::{
    bech32::{Bech32Address, Bech32ContractId},
    errors::{Error, Result},
    param_types::ParamType,
    parameters::{CallParameters, TxParameters},
    traits::Tokenizable,
    transaction::ScriptTransaction,
    Selector, Token,
};

use crate::calls::call_utils::simulate_and_validate;
use crate::{
    calls::call_response::FuelCallResponse,
    calls::call_utils::get_decoded_output,
    calls::contract_call_utils::{
        build_tx_from_contract_calls,
    },
    logs::{map_revert_error, LogDecoder},
};

/// How many times to attempt to resolve missing tx dependencies.
pub const DEFAULT_TX_DEP_ESTIMATION_ATTEMPTS: u64 = 10;

// Trait implemented by contract instances so that
// they can be passed to the `set_contracts` method
pub trait SettableContract {
    fn id(&self) -> Bech32ContractId;
    fn log_decoder(&self) -> LogDecoder;
}

#[derive(Debug)]
/// Contains all data relevant to a single contract call
pub struct ContractCall {
    pub contract_id: Bech32ContractId,
    pub encoded_args: UnresolvedBytes,
    pub encoded_selector: Selector,
    pub call_parameters: CallParameters,
    pub compute_custom_input_offset: bool,
    pub outputs: Vec<Output>,
    pub external_contracts: Vec<Bech32ContractId>,
    pub output_param: ParamType,
    pub is_payable: bool,
    pub custom_assets: HashMap<(AssetId, Option<Bech32Address>), u64>,
}

impl ContractCall {
    pub fn with_contract_id(self, contract_id: Bech32ContractId) -> Self {
        ContractCall {
            contract_id,
            ..self
        }
    }

    pub fn with_external_contracts(
        self,
        external_contracts: Vec<Bech32ContractId>,
    ) -> ContractCall {
        ContractCall {
            external_contracts,
            ..self
        }
    }

    pub fn with_call_parameters(self, call_parameters: CallParameters) -> ContractCall {
        ContractCall {
            call_parameters,
            ..self
        }
    }

    pub fn append_variable_outputs(&mut self, num: u64) {
        let new_variable_outputs = vec![
            Output::Variable {
                amount: 0,
                to: Address::zeroed(),
                asset_id: AssetId::default(),
            };
            num as usize
        ];
        self.outputs.extend(new_variable_outputs)
    }

    pub fn append_external_contracts(&mut self, contract_id: Bech32ContractId) {
        self.external_contracts.push(contract_id)
    }

    pub fn append_message_outputs(&mut self, num: u64) {
        let new_message_outputs = vec![
            Output::Message {
                recipient: Address::zeroed(),
                amount: 0,
            };
            num as usize
        ];
        self.outputs.extend(new_message_outputs)
    }

    fn is_missing_output_variables(receipts: &[Receipt]) -> bool {
        receipts.iter().any(
            |r| matches!(r, Receipt::Revert { ra, .. } if *ra == FAILED_TRANSFER_TO_ADDRESS_SIGNAL),
        )
    }

    fn find_contract_not_in_inputs(receipts: &[Receipt]) -> Option<&Receipt> {
        receipts.iter().find(
            |r| matches!(r, Receipt::Panic { reason, .. } if *reason.reason() == PanicReason::ContractNotInInputs ),
        )
    }

    pub fn add_custom_asset(&mut self, asset_id: AssetId, amount: u64, to: Option<Bech32Address>) {
        *self.custom_assets.entry((asset_id, to)).or_default() += amount;
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

    pub fn is_payable(&self) -> bool {
        self.contract_call.is_payable
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

    /// Sets the call parameters for a given contract call. Will fail if the call params forward
    /// some non-zero amount to a non-payable method.
    /// Note that this is a builder method, i.e. use it as a chain:
    ///
    /// ```ignore
    /// let params = CallParameters { amount: 1, asset_id: BASE_ASSET_ID };
    /// my_contract_instance.my_method(...).call_params(params).call()
    /// ```
    pub fn call_params(mut self, params: CallParameters) -> Result<Self> {
        if !self.is_payable() && params.amount > 0 {
            return Err(Error::AssetsForwardedToNonPayableMethod);
        }
        self.contract_call.call_parameters = params;
        Ok(self)
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

    /// Returns the script that executes the contract call
    pub async fn build_tx(&self) -> Result<ScriptTransaction> {
        build_tx_from_contract_calls(
            std::slice::from_ref(&self.contract_call),
            &self.tx_parameters,
            &self.wallet,
        )
        .await
    }

    /// Call a contract's method on the node, in a state-modifying manner.
    pub async fn call(self) -> Result<FuelCallResponse<D>> {
        self.call_or_simulate(false)
            .await
            .map_err(|err| map_revert_error(err, &self.log_decoder))
    }

    /// Call a contract's method on the node, in a simulated manner, meaning the state of the
    /// blockchain is *not* modified but simulated.
    /// It is the same as the [`call`] method because the API is more user-friendly this way.
    ///
    /// [`call`]: Self::call
    pub async fn simulate(&self) -> Result<FuelCallResponse<D>> {
        self.call_or_simulate(true)
            .await
            .map_err(|err| map_revert_error(err, &self.log_decoder))
    }

    async fn call_or_simulate(&self, simulate: bool) -> Result<FuelCallResponse<D>> {
        let tx = self.build_tx().await?;

        let receipts = if simulate {
            simulate_and_validate(&self.provider, &tx).await?
        } else {
            self.provider.send_transaction(&tx).await?
        };

        self.get_response(receipts)
    }

    /// Simulates the call and attempts to resolve missing tx dependencies.
    /// Forwards the received error if it cannot be fixed.
    pub async fn estimate_tx_dependencies(mut self, max_attempts: Option<u64>) -> Result<Self> {
        let attempts = max_attempts.unwrap_or(DEFAULT_TX_DEP_ESTIMATION_ATTEMPTS);

        for _ in 0..attempts {
            let result = self.simulate().await;

            match result {
                Err(Error::RevertTransactionError { receipts, .. })
                    if ContractCall::is_missing_output_variables(&receipts) =>
                {
                    self = self.append_variable_outputs(1);
                }

                Err(Error::RevertTransactionError { ref receipts, .. }) => {
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
        let script = self.build_tx().await?;

        let transaction_cost = self
            .provider
            .estimate_transaction_cost(&script, tolerance)
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
    pub async fn build_tx(&self) -> Result<ScriptTransaction> {
        if self.contract_calls.is_empty() {
            panic!("No calls added. Have you used '.add_calls()'?");
        }

        build_tx_from_contract_calls(&self.contract_calls, &self.tx_parameters, &self.wallet).await
    }

    /// Call contract methods on the node, in a state-modifying manner.
    pub async fn call<D: Tokenizable + Debug>(&self) -> Result<FuelCallResponse<D>> {
        self.call_or_simulate(false)
            .await
            .map_err(|err| map_revert_error(err, &self.log_decoder))
    }

    /// Call contract methods on the node, in a simulated manner, meaning the state of the
    /// blockchain is *not* modified but simulated.
    /// It is the same as the [call] method because the API is more user-friendly this way.
    ///
    /// [call]: Self::call
    pub async fn simulate<D: Tokenizable + Debug>(&self) -> Result<FuelCallResponse<D>> {
        self.call_or_simulate(true)
            .await
            .map_err(|err| map_revert_error(err, &self.log_decoder))
    }

    async fn call_or_simulate<D: Tokenizable + Debug>(
        &self,
        simulate: bool,
    ) -> Result<FuelCallResponse<D>> {
        let provider = self.wallet.get_provider()?;
        let tx = self.build_tx().await?;

        let receipts = if simulate {
            simulate_and_validate(provider, &tx).await?
        } else {
            provider.send_transaction(&tx).await?
        };

        self.get_response(receipts)
    }

    /// Simulates a call without needing to resolve the generic for the return type
    async fn simulate_without_decode(&self) -> Result<()> {
        let provider = self.wallet.get_provider()?;
        let tx = self.build_tx().await?;

        simulate_and_validate(provider, &tx).await?;

        Ok(())
    }

    /// Simulates the call and attempts to resolve missing tx dependencies.
    /// Forwards the received error if it cannot be fixed.
    pub async fn estimate_tx_dependencies(mut self, max_attempts: Option<u64>) -> Result<Self> {
        let attempts = max_attempts.unwrap_or(DEFAULT_TX_DEP_ESTIMATION_ATTEMPTS);

        for _ in 0..attempts {
            let result = self.simulate_without_decode().await;

            match result {
                Err(Error::RevertTransactionError { receipts, .. })
                    if ContractCall::is_missing_output_variables(&receipts) =>
                {
                    self.contract_calls
                        .iter_mut()
                        .take(1)
                        .for_each(|call| call.append_variable_outputs(1));
                }

                Err(Error::RevertTransactionError { ref receipts, .. }) => {
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
        let script = self.build_tx().await?;

        let transaction_cost = self
            .wallet
            .get_provider()?
            .estimate_transaction_cost(&script, tolerance)
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
