use std::fmt::Debug;

use fuel_types::Bytes32;
use fuels_accounts::Account;
use fuels_core::{
    traits::{Parameterize, Tokenizable},
    types::errors::Result,
};

use crate::{
    calls::{
        CallHandler, ContractCall,
        traits::{ContractDependencyConfigurator, ResponseParser, TransactionTuner},
    },
    responses::CallResponse,
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
/// - `C`: The call type.
///
/// # Fields
///
/// - `retry_config`: The retry configuration for the transaction.
/// - `tx_id`: The optional transaction ID of the submitted transaction.
/// - `call_handler`: The call handler that manages the type of call.
///
/// ```
#[derive(Debug)]
pub struct SubmitResponse<A, C, T> {
    tx_id: Bytes32,
    call_handler: CallHandler<A, C, T>,
}

impl<A, C, T> SubmitResponse<A, C, T>
where
    A: Account,
    C: ContractDependencyConfigurator + TransactionTuner + ResponseParser,
    T: Tokenizable + Parameterize + Debug,
{
    pub fn new(tx_id: Bytes32, call_handler: CallHandler<A, C, T>) -> Self {
        Self {
            tx_id,
            call_handler,
        }
    }

    pub async fn response(self) -> Result<CallResponse<T>> {
        let provider = self.call_handler.account.try_provider()?;
        let tx_status = provider.tx_status(&self.tx_id).await?;

        self.call_handler.get_response(tx_status)
    }

    pub fn tx_id(&self) -> Bytes32 {
        self.tx_id
    }
}

/// Represents the response of a submitted transaction with multiple contract calls.
impl<A: Account> SubmitResponse<A, Vec<ContractCall>, ()> {
    pub fn new(tx_id: Bytes32, call_handler: CallHandler<A, Vec<ContractCall>, ()>) -> Self {
        Self {
            tx_id,
            call_handler,
        }
    }

    pub async fn response<T: Tokenizable + Debug>(self) -> Result<CallResponse<T>> {
        let provider = self.call_handler.account.try_provider()?;
        let tx_status = provider.tx_status(&self.tx_id).await?;

        self.call_handler.get_response(tx_status)
    }

    pub fn tx_id(&self) -> Bytes32 {
        self.tx_id
    }
}
