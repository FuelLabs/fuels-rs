use std::fmt::Debug;

use fuel_tx::Receipt;
use fuel_types::Bytes32;
use fuels_accounts::{provider::Provider, Account};
use fuels_core::{
    codec::LogDecoder,
    traits::{Parameterize, Tokenizable},
    types::errors::Result,
};

use crate::{
    call_response::FuelCallResponse,
    contract::{ContractCallHandler, MultiContractCallHandler},
    script_calls::ScriptCallHandler,
};

/// Represents the response of a submitted transaction with customizable retry behavior.
///
/// This struct holds information about the retry configuration, transaction ID (`tx_id`),
/// and the call handler that manages the type of call (contract or script).
///
/// # Type Parameters
///
/// - `T`: The account type associated with the transaction.
/// - `D`: The data type representing the response value.
///
/// # Fields
///
/// - `retry_config`: The retry configuration for the transaction.
/// - `tx_id`: The optional transaction ID of the submitted transaction.
/// - `call_handler`: The call handler that manages the type of call.
///
/// ```
#[derive(Debug)]
pub struct SubmitResponse<T: Account, D> {
    tx_id: Bytes32,
    call_handler: CallHandler<T, D>,
}

#[derive(Debug)]
pub enum CallHandler<T: Account, D> {
    Contract(ContractCallHandler<T, D>),
    Script(ScriptCallHandler<T, D>),
}

impl<T: Account, D> From<ScriptCallHandler<T, D>> for CallHandler<T, D> {
    fn from(value: ScriptCallHandler<T, D>) -> Self {
        Self::Script(value)
    }
}

impl<T: Account, D> From<ContractCallHandler<T, D>> for CallHandler<T, D> {
    fn from(value: ContractCallHandler<T, D>) -> Self {
        Self::Contract(value)
    }
}

impl<T, D> CallHandler<T, D>
where
    T: Account,
    D: Tokenizable + Parameterize + Debug,
{
    fn get_response(&self, receipts: Vec<Receipt>) -> Result<FuelCallResponse<D>> {
        match self {
            CallHandler::Contract(contract_handler) => contract_handler.get_response(receipts),
            CallHandler::Script(script_handler) => script_handler.get_response(receipts),
        }
    }

    fn try_provider(&self) -> Result<&Provider> {
        let account = match self {
            CallHandler::Contract(contract_handler) => &contract_handler.account,
            CallHandler::Script(script_handler) => &script_handler.account,
        };

        account.try_provider()
    }

    fn log_decoder(&self) -> &LogDecoder {
        match self {
            CallHandler::Contract(handler) => &handler.log_decoder,
            CallHandler::Script(handler) => &handler.log_decoder,
        }
    }
}

impl<T: Account, D: Tokenizable + Parameterize + Debug> SubmitResponse<T, D> {
    pub fn new(tx_id: Bytes32, call_handler: impl Into<CallHandler<T, D>>) -> Self {
        Self {
            tx_id,
            call_handler: call_handler.into(),
        }
    }

    pub async fn response(self) -> Result<FuelCallResponse<D>> {
        let provider = self.call_handler.try_provider()?;
        let receipts = provider
            .tx_status(&self.tx_id)
            .await?
            .take_receipts_checked(Some(self.call_handler.log_decoder()))?;

        self.call_handler.get_response(receipts)
    }

    pub fn tx_id(&self) -> Bytes32 {
        self.tx_id
    }
}

/// Represents the response of a submitted transaction with multiple contract calls.
///
/// This struct is similar to `SubmitResponse` but is designed to handle transactions
/// with multiple contract calls.
#[derive(Debug)]
pub struct SubmitResponseMultiple<T: Account> {
    tx_id: Bytes32,
    call_handler: MultiContractCallHandler<T>,
}

impl<T: Account> SubmitResponseMultiple<T> {
    pub fn new(tx_id: Bytes32, call_handler: MultiContractCallHandler<T>) -> Self {
        Self {
            tx_id,
            call_handler,
        }
    }

    pub async fn response<D: Tokenizable + Debug>(self) -> Result<FuelCallResponse<D>> {
        let provider = self.call_handler.account.try_provider()?;
        let receipts = provider
            .tx_status(&self.tx_id)
            .await?
            .take_receipts_checked(Some(&self.call_handler.log_decoder))?;

        self.call_handler.get_response(receipts)
    }

    pub fn tx_id(&self) -> Bytes32 {
        self.tx_id
    }
}
