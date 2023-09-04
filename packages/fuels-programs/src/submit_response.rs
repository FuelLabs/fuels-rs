use crate::call_response::FuelCallResponse;
use crate::contract::{ContractCallHandler, MultiContractCallHandler};
use crate::retry::{retry, RetryConfig};
use crate::script_calls::ScriptCallHandler;
use fuel_tx::Receipt;
use fuel_types::Bytes32;
use fuels_accounts::provider::Provider;
use fuels_accounts::Account;
use fuels_core::traits::{Parameterize, Tokenizable};
use fuels_core::types::errors;
use fuels_core::types::errors::{Error, Result};
use std::fmt::Debug;

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
#[derive(Debug, Clone)]
pub struct SubmitResponse<T: Account, D> {
    pub retry_config: RetryConfig,
    pub tx_id: Option<Bytes32>,
    pub call_handler: CallHandler<T, D>,
}

#[derive(Debug, Clone)]
pub enum CallHandler<T: Account, D> {
    Contract(ContractCallHandler<T, D>),
    Script(ScriptCallHandler<T, D>),
}

pub trait ResponseHandler<T, D> {
    fn get_response(&self, receipts: Vec<Receipt>) -> Result<FuelCallResponse<D>>;
    fn try_provider(&self) -> Result<&Provider>;
}

impl<T, D> ResponseHandler<T, D> for CallHandler<T, D>
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
        Ok(account.try_provider()?)
    }
}

impl<T: Account, D: Tokenizable + Parameterize + Debug> SubmitResponse<T, D> {
    pub fn new(tx_id: Option<Bytes32>, call_handler: CallHandler<T, D>) -> Self {
        Self {
            retry_config: Default::default(),
            tx_id,
            call_handler,
        }
    }

    pub fn with_retry_config(mut self, retry_config: RetryConfig) -> Self {
        self.retry_config = retry_config;
        self
    }

    pub async fn value(self) -> Result<D> {
        self.process_value().await
    }

    async fn process_value(self) -> Result<D> {
        let provider = self.call_handler.try_provider()?;

        let should_retry_fn = |res: &errors::Result<Option<Vec<Receipt>>>| -> bool {
            match res {
                Err(err) if matches!(err, Error::IOError(_)) => true,
                Ok(None) => true,
                _ => false,
            }
        };

        let receipts = if self.retry_config.max_attempts != 0 {
            retry(
                || async {
                    provider
                        .client
                        .receipts(&self.tx_id.expect("tx_id is missing"))
                        .await
                        .map_err(|e| e.into())
                },
                &self.retry_config,
                should_retry_fn,
            )
            .await?
        } else {
            provider
                .client
                .receipts(&self.tx_id.expect("tx_id is missing"))
                .await?
        };

        let value = self.call_handler.get_response(receipts.unwrap())?.value;
        Ok(value)
    }
}

/// Represents the response of a submitted transaction with multiple contract calls.
///
/// This struct is similar to `SubmitResponse` but is designed to handle transactions
/// with multiple contract calls.
#[derive(Debug, Clone)]
pub struct SubmitResponseMultiple<T: Account> {
    pub retry_config: RetryConfig,
    pub tx_id: Option<Bytes32>,
    pub call_handler: MultiContractCallHandler<T>,
}

impl<T: Account> SubmitResponseMultiple<T> {
    pub fn new(tx_id: Option<Bytes32>, call_handler: MultiContractCallHandler<T>) -> Self {
        Self {
            retry_config: Default::default(),
            tx_id,
            call_handler,
        }
    }

    pub fn with_retry_config(mut self, retry_config: RetryConfig) -> Self {
        self.retry_config = retry_config;
        self
    }

    pub async fn value<D: Tokenizable + Debug>(self) -> errors::Result<D> {
        self.process_value().await
    }

    async fn process_value<D: Tokenizable + Debug>(self) -> errors::Result<D> {
        let provider = self.call_handler.account.try_provider()?;

        let should_retry_fn = |res: &errors::Result<Option<Vec<Receipt>>>| -> bool {
            match res {
                Err(err) if matches!(err, Error::IOError(_)) => true,
                Ok(None) => true,
                _ => false,
            }
        };

        let receipts = if self.retry_config.max_attempts != 0 {
            retry(
                || async {
                    provider
                        .client
                        .receipts(&self.tx_id.expect("tx_id is missing"))
                        .await
                        .map_err(|e| e.into())
                },
                &self.retry_config,
                should_retry_fn,
            )
            .await?
        } else {
            provider
                .client
                .receipts(&self.tx_id.expect("tx_id is missing"))
                .await?
        };

        let value = self.call_handler.get_response(receipts.unwrap())?.value;
        Ok(value)
    }
}
