#![allow(unused_imports)]
#![allow(unused)]
use std::fmt::Debug;
use std::marker::PhantomData;

use fuel_gql_client::{
    fuel_tx::{Output, Receipt, Transaction},
    fuel_types::{Address, AssetId, Salt},
};

use crate::contract::{get_decoded_output, DEFAULT_TX_DEP_ESTIMATION_ATTEMPTS};
use fuels_core::abi_encoder::{ABIEncoder, UnresolvedBytes};
use fuels_core::constants::FAILED_TRANSFER_TO_ADDRESS_SIGNAL;
use fuels_core::tx::{Bytes32, ContractId};
use fuels_core::{
    parameters::{CallParameters, TxParameters},
    Parameterize, Token, Tokenizable,
};
use fuels_signers::{
    provider::{Provider, TransactionCost},
    WalletUnlocked,
};
use fuels_types::bech32::Bech32ContractId;
use fuels_types::{
    errors::Error,
    param_types::{ParamType, ReturnLocation},
};

use crate::execution_script::{CompiledScript, TransactionExecution};

//
//
//
//```rust
//#[derive(Debug)]
//SCRIPTS
//pub struct #name{
//wallet: WalletUnlocked,
//binary_filepath: String,
//}
//
//
//
//
//pub struct #name {
//contract_id: Bech32ContractId,
//wallet: WalletUnlocked,
//logs_lookup: Vec<(u64, ParamType)>,
//}
//```
//
//

pub struct ScriptInterface {
    pub script: TransactionExecution,
    pub wallet: WalletUnlocked,
}

#[derive(Debug)]
pub struct ScriptCallResponse<D> {
    pub value: D,
    pub receipts: Vec<Receipt>,
    pub gas_used: u64,
}

impl<D> ScriptCallResponse<D> {
    /// Get the gas used from ScriptResult receipt
    fn get_gas_used(receipts: &[Receipt]) -> u64 {
        receipts
            .iter()
            .rfind(|r| matches!(r, Receipt::ScriptResult { .. }))
            .expect("could not retrieve ScriptResult")
            .gas_used()
            .expect("could not retrieve gas used from ScriptResult")
    }

    pub fn new(value: D, receipts: Vec<Receipt>) -> Self {
        Self {
            value,
            gas_used: Self::get_gas_used(&receipts),
            receipts,
        }
    }
}

#[derive(Debug)]
/// Contains all data relevant to a single contract call
pub struct ScriptCall {
    pub script: TransactionExecution,
    // pub encoded_args: UnresolvedBytes,
    // pub call_parameters: CallParameters,
    // pub compute_custom_input_offset: bool,
    // pub variable_outputs: Option<Vec<Output>>,
    // pub message_outputs: Option<Vec<Output>>,
    // pub external_contracts: Vec<Bech32ContractId>,
    // pub output_param: ParamType,
}

// impl ScriptCall {
// pub fn with_external_contracts(self, external_contracts: Vec<Bech32ContractId>) -> ScriptCall {
//     ScriptCall {
//         external_contracts,
//         ..self
//     }
// }

// pub fn with_variable_outputs(self, variable_outputs: Vec<Output>) -> ScriptCall {
//     ScriptCall {
//         variable_outputs: Some(variable_outputs),
//         ..self
//     }
// }
//
// pub fn with_message_outputs(self, message_outputs: Vec<Output>) -> ScriptCall {
//     ScriptCall {
//         message_outputs: Some(message_outputs),
//         ..self
//     }
// }
//
// pub fn with_call_parameters(self, call_parameters: CallParameters) -> ScriptCall {
//     ScriptCall {
//         call_parameters,
//         ..self
//     }
// }
//
// pub fn append_variable_outputs(&mut self, num: u64) {
//     let new_variable_outputs = vec![
//         Output::Variable {
//             amount: 0,
//             to: Address::zeroed(),
//             asset_id: AssetId::default(),
//         };
//         num as usize
//     ];
//
//     match self.variable_outputs {
//         Some(ref mut outputs) => outputs.extend(new_variable_outputs),
//         None => self.variable_outputs = Some(new_variable_outputs),
//     }
// }
//
// pub fn append_message_outputs(&mut self, num: u64) {
//     let new_message_outputs = vec![
//         Output::Message {
//             recipient: Address::zeroed(),
//             amount: 0,
//         };
//         num as usize
//     ];
//
//     match self.message_outputs {
//         Some(ref mut outputs) => outputs.extend(new_message_outputs),
//         None => self.message_outputs = Some(new_message_outputs),
//     }
// }
//
// fn is_missing_output_variables(receipts: &[Receipt]) -> bool {
//     receipts.iter().any(
//         |r| matches!(r, Receipt::Revert { ra, .. } if *ra == FAILED_TRANSFER_TO_ADDRESS_SIGNAL),
//     )
// }
// }

#[derive(Debug)]
#[must_use = "script calls do nothing unless you `call` them"]
/// Helper that handles submitting a script call to a client and formatting the response
pub struct ScriptCallHandler<D> {
    pub compiled_script: CompiledScript,
    pub script_data: Vec<u8>,
    pub tx_parameters: TxParameters,
    pub wallet: WalletUnlocked,
    pub provider: Provider,
    pub output_param: ParamType,
    pub datatype: PhantomData<D>,
}

impl<D> ScriptCallHandler<D>
where
    D: Tokenizable + Debug,
{
    /// Sets external contracts as dependencies to this script's call.
    /// Effectively, this will be used to create Input::Contract/Output::Contract
    /// pairs and set them into the transaction.
    /// Note that this is a builder method, i.e. use it as a chain:
    /// `my_contract_instance.my_method(...).set_contracts(&[another_contract_id]).call()`.
    // pub fn set_contracts(mut self, contract_ids: &[Bech32ContractId]) -> Self {
    //     self.compiled_script.external_contracts = contract_ids.to_vec();
    //     self
    // }

    /// Sets the transaction parameters for a given transaction.
    /// Note that this is a builder method, i.e. use it as a chain:
    /// let params = TxParameters { gas_price: 100, gas_limit: 1000000 };
    /// `my_contract_instance.my_method(...).tx_params(params).call()`.
    pub fn tx_params(mut self, params: TxParameters) -> Self {
        self.tx_parameters = params;
        self
    }

    /// Sets the call parameters for a given contract call.
    /// Note that this is a builder method, i.e. use it as a chain:
    /// let params = CallParameters { amount: 1, asset_id: BASE_ASSET_ID };
    /// `my_contract_instance.my_method(...).call_params(params).call()`.
    // pub fn call_params(mut self, params: CallParameters) -> Self {
    //     self.compiled_script.call_parameters = params;
    //     self
    // }

    /// Appends `num` `Output::Variable`s to the transaction.
    /// Note that this is a builder method, i.e. use it as a chain:
    /// `my_contract_instance.my_method(...).add_variable_outputs(num).call()`.
    // pub fn append_variable_outputs(mut self, num: u64) -> Self {
    //     self.compiled_script.append_variable_outputs(num);
    //     self
    // }

    /// Appends `num` `Output::Message`s to the transaction.
    /// Note that this is a builder method, i.e. use it as a chain:
    /// `my_contract_instance.my_method(...).add_message_outputs(num).call()`.
    // pub fn append_message_outputs(mut self, num: u64) -> Self {
    //     self.compiled_script.append_message_outputs(num);
    //     self
    // }

    /// Call a contract's method on the node. If `simulate==true`, then the call is done in a
    /// read-only manner, using a `dry-run`. Return a Result<CallResponse, Error>. The CallResponse
    /// struct contains the method's value in its `value` field as an actual typed value `D` (if
    /// your method returns `bool`, it will be a bool, works also for structs thanks to the
    /// `abigen!()`). The other field of CallResponse, `receipts`, contains the receipts of the
    /// transaction.
    #[tracing::instrument]
    async fn call_or_simulate(&self, simulate: bool) -> Result<ScriptCallResponse<D>, Error> {
        let tx = Transaction::script(
            self.tx_parameters.gas_price,
            self.tx_parameters.gas_limit,
            self.tx_parameters.maturity,
            self.compiled_script.script_binary.clone(),
            self.script_data.clone(),
            vec![], //TODO inputs
            vec![], //TODO outputs
            vec![], //TODO witnesses
        );

        let tx_execution = TransactionExecution { tx };
        let receipts = if simulate {
            tx_execution.simulate(&self.provider).await?
        } else {
            tx_execution.execute(&self.provider).await?
        };
        tracing::debug!(target: "receipts", "{:?}", receipts);

        self.get_response(receipts)
    }

    /// Call a contract's method on the node, in a state-modifying manner.
    pub async fn call(self) -> Result<ScriptCallResponse<D>, Error> {
        Self::call_or_simulate(&self, false).await
    }

    /// Call a contract's method on the node, in a simulated manner, meaning the state of the
    /// blockchain is *not* modified but simulated.
    /// It is the same as the `call` method because the API is more user-friendly this way.
    pub async fn simulate(self) -> Result<ScriptCallResponse<D>, Error> {
        Self::call_or_simulate(&self, true).await
    }

    // /// Simulates the call and attempts to resolve missing tx dependencies.
    // /// Forwards the received error if it cannot be fixed.
    // pub async fn estimate_tx_dependencies(
    //     mut self,
    //     max_attempts: Option<u64>,
    // ) -> Result<Self, Error> {
    //     let attempts = max_attempts.unwrap_or(DEFAULT_TX_DEP_ESTIMATION_ATTEMPTS);
    //
    //     for _ in 0..attempts {
    //         let result = self.call_or_simulate(true).await;
    //
    //         match result {
    //             Err(Error::RevertTransactionError(_, receipts))
    //                 if ScriptCall::is_missing_output_variables(&receipts) =>
    //             {
    //                 self = self.append_variable_outputs(1);
    //             }
    //             Err(e) => return Err(e),
    //             _ => return Ok(self),
    //         }
    //     }
    //
    //     // confirm if successful or propagate error
    //     match self.call_or_simulate(true).await {
    //         Ok(_) => Ok(self),
    //         Err(e) => Err(e),
    //     }
    // }

    // /// Get a contract's estimated cost
    // pub async fn estimate_transaction_cost(
    //     &self,
    //     tolerance: Option<f64>,
    // ) -> Result<TransactionCost, Error> {
    //     let transaction_cost = self
    //         .provider
    //         .estimate_transaction_cost(&self.compiled_script.script.tx, tolerance)
    //         .await?;
    //
    //     Ok(transaction_cost)
    // }

    /// Create a CallResponse from call receipts
    pub fn get_response(&self, mut receipts: Vec<Receipt>) -> Result<ScriptCallResponse<D>, Error> {
        let token = get_decoded_output(&mut receipts, None, &self.output_param)?;
        Ok(ScriptCallResponse::new(D::from_token(token)?, receipts))
    }
}
