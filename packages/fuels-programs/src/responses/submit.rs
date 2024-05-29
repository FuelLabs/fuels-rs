use std::fmt::Debug;

use fuel_types::Bytes32;
use fuels_accounts::Account;
use fuels_core::{
    traits::{Parameterize, Tokenizable},
    types::errors::Result,
};

use crate::{
    call_handler::CallHandler,
    calls::{
        traits::{Buildable, Extendable, Parsable},
        ContractCall,
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
pub struct SubmitResponse<T, D, C> {
    tx_id: Bytes32,
    call_handler: CallHandler<T, D, C>,
}

impl<T, D, C> SubmitResponse<T, D, C>
where
    T: Account,
    D: Tokenizable + Parameterize + Debug,
    C: Extendable + Buildable + Parsable,
{
    pub fn new(tx_id: Bytes32, call_handler: CallHandler<T, D, C>) -> Self {
        Self {
            tx_id,
            call_handler,
        }
    }

    pub async fn response(self) -> Result<CallResponse<D>> {
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

/// Represents the response of a submitted transaction with multiple contract calls.
impl<T: Account> SubmitResponse<T, (), Vec<ContractCall>> {
    pub fn new(tx_id: Bytes32, call_handler: CallHandler<T, (), Vec<ContractCall>>) -> Self {
        Self {
            tx_id,
            call_handler,
        }
    }

    pub async fn response<D: Tokenizable + Debug>(self) -> Result<CallResponse<D>> {
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
