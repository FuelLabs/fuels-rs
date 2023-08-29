use fuel_tx::Receipt;
use fuel_types::Bytes32;
use fuels_accounts::{Account, ViewOnlyAccount};
use fuels_accounts::provider::Provider;
use fuels_core::traits::{Parameterize, Tokenizable};
use fuels_core::types::errors::{Error, Result};
use std::fmt::Debug;
use fuels_core::types::errors;
use crate::call_response::FuelCallResponse;
use crate::contract::{ContractCallHandler, MultiContractCallHandler};
use crate::retry::{retry, RetryConfig};
use crate::script_calls::ScriptCallHandler;


pub struct SubmitResponse<T: Account, D> {
    pub retry_config: RetryConfig, // Use RetryConfig<D> instead of RetryConfig
    pub tx_id: Option<Bytes32>,
    pub call_handler: CallHandler<T, D>,
}

// Define the enum for different contract call handler types
pub enum CallHandler<T: Account, D> {
    Contract(ContractCallHandler<T, D>),
    MultiContract(MultiContractCallHandler<T>),
    Script(ScriptCallHandler<T, D>),
}

pub trait ResponseHandler<T, D> {
    fn get_response(&self, receipts: Vec<Receipt>) -> Result<FuelCallResponse<D>>;
    fn try_provider(&self) -> Result<&Provider>;
}

impl<T, D> ResponseHandler<T, D> for CallHandler<T, D>
where
    T: Account + 'static,
    D: Tokenizable + Parameterize + Debug + 'static,
{
    fn get_response(&self, receipts: Vec<Receipt>) -> Result<FuelCallResponse<D>> {
        match self {
            CallHandler::Contract(contract_handler) => contract_handler.get_response(receipts),
            CallHandler::MultiContract(multi_contract_handler) => {
                multi_contract_handler.get_response(receipts)
            }
            CallHandler::Script(script_handler) => script_handler.get_response(receipts),
        }
    }

    fn try_provider(&self) -> Result<&Provider> {
        let account = match self {
            CallHandler::Contract(contract_handler) => &contract_handler.account,
            CallHandler::MultiContract(multi_contract_handler) => &multi_contract_handler.account,
            CallHandler::Script(script_handler) => &script_handler.account,
        };
        Ok(account.try_provider()?)
    }
}

impl<T: Account + 'static, D: Tokenizable + Parameterize + Debug + 'static> SubmitResponse<T, D> {
    pub fn new(tx_id: Option<Bytes32>, call_handler: CallHandler<T, D>) -> Self {
        Self {
            retry_config: Default::default(),
            tx_id,
            call_handler,
        }
    }

    pub fn retry_config(mut self, retry_config: RetryConfig) -> Self {
        self.retry_config = retry_config;
        self
    }

    pub async fn value(self) -> errors::Result<D> {
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
