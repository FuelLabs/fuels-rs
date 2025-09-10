use fuel_abi_types::error_codes::{
    FAILED_ASSERT_EQ_SIGNAL, FAILED_ASSERT_NE_SIGNAL, FAILED_ASSERT_SIGNAL, FAILED_REQUIRE_SIGNAL,
    FAILED_SEND_MESSAGE_SIGNAL, FAILED_TRANSFER_TO_ADDRESS_SIGNAL, REVERT_WITH_LOG_SIGNAL,
};
#[cfg(feature = "std")]
use fuel_core_client::client::types::TransactionStatus as ClientTransactionStatus;
#[cfg(feature = "std")]
use fuel_core_types::services::executor::{TransactionExecutionResult, TransactionExecutionStatus};
use fuel_tx::Receipt;
#[cfg(feature = "std")]
use fuel_vm::state::ProgramState;

use crate::{
    codec::LogDecoder,
    types::errors::{Error, Result, transaction::Reason},
};

#[derive(Debug, Clone)]
pub struct Success {
    pub receipts: Vec<Receipt>,
    pub total_fee: u64,
    pub total_gas: u64,
}

#[derive(Debug, Clone)]
pub struct SqueezedOut {
    pub reason: String,
}

#[derive(Debug, Clone)]
pub struct Failure {
    pub reason: String,
    pub receipts: Vec<Receipt>,
    pub revert_id: Option<u64>,
    pub total_fee: u64,
    pub total_gas: u64,
}

#[derive(Debug, Clone)]
pub enum TxStatus {
    Success(Success),
    PreconfirmationSuccess(Success),
    Submitted,
    SqueezedOut(SqueezedOut),
    Failure(Failure),
    PreconfirmationFailure(Failure),
}

impl TxStatus {
    pub fn check(&self, log_decoder: Option<&LogDecoder>) -> Result<()> {
        match self {
            Self::SqueezedOut(SqueezedOut { reason }) => {
                Err(Error::Transaction(Reason::SqueezedOut(reason.clone())))
            }
            Self::Failure(Failure {
                receipts,
                reason,
                revert_id,
                ..
            }) => Err(Self::map_revert_error(
                receipts.clone(),
                reason,
                *revert_id,
                log_decoder,
            )),
            _ => Ok(()),
        }
    }

    pub fn take_success_checked(self, log_decoder: Option<&LogDecoder>) -> Result<Success> {
        match self {
            Self::SqueezedOut(SqueezedOut { reason }) => {
                Err(Error::Transaction(Reason::SqueezedOut(reason.clone())))
            }
            Self::Failure(Failure {
                receipts,
                reason,
                revert_id,
                ..
            })
            | Self::PreconfirmationFailure(Failure {
                receipts,
                reason,
                revert_id,
                ..
            }) => Err(Self::map_revert_error(
                receipts,
                &reason,
                revert_id,
                log_decoder,
            )),
            Self::Submitted => Err(Error::Transaction(Reason::Other(
                "transactions was not yet included".to_owned(),
            ))),
            Self::Success(success) | Self::PreconfirmationSuccess(success) => Ok(success),
        }
    }

    pub fn total_gas(&self) -> u64 {
        match self {
            TxStatus::Success(Success { total_gas, .. })
            | TxStatus::Failure(Failure { total_gas, .. }) => *total_gas,
            _ => 0,
        }
    }

    pub fn total_fee(&self) -> u64 {
        match self {
            TxStatus::Success(Success { total_fee, .. })
            | TxStatus::Failure(Failure { total_fee, .. }) => *total_fee,
            _ => 0,
        }
    }

    fn map_revert_error(
        receipts: Vec<Receipt>,
        reason: &str,
        revert_id: Option<u64>,
        log_decoder: Option<&LogDecoder>,
    ) -> Error {
        if let (Some(revert_id), Some(log_decoder)) = (revert_id, log_decoder) {
            if let Some(error_detail) = log_decoder.get_error_codes(&revert_id) {
                let error_message = if error_detail.log_id.is_some() {
                    log_decoder
                        .decode_last_log(&receipts)
                        .unwrap_or_else(|err| {
                            format!("failed to decode log from require revert: {err}")
                        })
                } else {
                    error_detail.msg.clone().expect("is there")
                };

                let reason = format!(
                    "panicked at: `{}` - `{}:{}:{}` with message `{}`",
                    error_detail.pkg,
                    error_detail.file,
                    error_detail.line,
                    error_detail.column,
                    error_message
                );

                return Error::Transaction(Reason::Failure {
                    reason,
                    revert_id: Some(revert_id),
                    receipts,
                });
            }
        }

        let reason = match (revert_id, log_decoder) {
            (Some(FAILED_REQUIRE_SIGNAL), Some(log_decoder)) => log_decoder
                .decode_last_log(&receipts)
                .unwrap_or_else(|err| format!("failed to decode log from require revert: {err}")),
            (Some(REVERT_WITH_LOG_SIGNAL), Some(log_decoder)) => log_decoder
                .decode_last_log(&receipts)
                .unwrap_or_else(|err| format!("failed to decode log from revert_with_log: {err}")),
            (Some(FAILED_ASSERT_EQ_SIGNAL), Some(log_decoder)) => {
                match log_decoder.decode_last_two_logs(&receipts) {
                    Ok((lhs, rhs)) => format!(
                        "assertion failed: `(left == right)`\n left: `{lhs:?}`\n right: `{rhs:?}`"
                    ),
                    Err(err) => {
                        format!("failed to decode log from assert_eq revert: {err}")
                    }
                }
            }
            (Some(FAILED_ASSERT_NE_SIGNAL), Some(log_decoder)) => {
                match log_decoder.decode_last_two_logs(&receipts) {
                    Ok((lhs, rhs)) => format!(
                        "assertion failed: `(left != right)`\n left: `{lhs:?}`\n right: `{rhs:?}`"
                    ),
                    Err(err) => {
                        format!("failed to decode log from assert_eq revert: {err}")
                    }
                }
            }
            (Some(FAILED_ASSERT_SIGNAL), _) => "assertion failed".into(),
            (Some(FAILED_SEND_MESSAGE_SIGNAL), _) => "failed to send message".into(),
            (Some(FAILED_TRANSFER_TO_ADDRESS_SIGNAL), _) => "failed transfer to address".into(),
            _ => reason.to_string(),
        };

        Error::Transaction(Reason::Failure {
            reason,
            revert_id,
            receipts,
        })
    }

    pub fn take_receipts_checked(self, log_decoder: Option<&LogDecoder>) -> Result<Vec<Receipt>> {
        self.check(log_decoder)?;
        Ok(self.take_receipts())
    }

    pub fn take_receipts(self) -> Vec<Receipt> {
        match self {
            TxStatus::Success(Success { receipts, .. })
            | TxStatus::Failure(Failure { receipts, .. }) => receipts,
            _ => vec![],
        }
    }

    pub fn is_final(&self) -> bool {
        matches!(
            self,
            TxStatus::Success(_) | TxStatus::Failure(_) | TxStatus::SqueezedOut(_)
        )
    }
}

#[cfg(feature = "std")]
impl From<ClientTransactionStatus> for TxStatus {
    fn from(client_status: ClientTransactionStatus) -> Self {
        match client_status {
            ClientTransactionStatus::Submitted { .. } => TxStatus::Submitted {},
            ClientTransactionStatus::Success {
                receipts,
                total_gas,
                total_fee,
                ..
            } => TxStatus::Success(Success {
                receipts,
                total_gas,
                total_fee,
            }),
            ClientTransactionStatus::PreconfirmationSuccess {
                receipts,
                total_gas,
                total_fee,
                ..
            } => TxStatus::PreconfirmationSuccess(Success {
                receipts: receipts.unwrap_or_default(),
                total_gas,
                total_fee,
            }),
            ClientTransactionStatus::Failure {
                reason,
                program_state,
                receipts,
                total_gas,
                total_fee,
                ..
            } => {
                let revert_id = program_state.and_then(|state| match state {
                    ProgramState::Revert(revert_id) => Some(revert_id),
                    _ => None,
                });

                TxStatus::Failure(Failure {
                    receipts,
                    reason,
                    revert_id,
                    total_gas,
                    total_fee,
                })
            }
            ClientTransactionStatus::PreconfirmationFailure {
                reason,
                receipts,
                total_gas,
                total_fee,
                ..
            } => TxStatus::Failure(Failure {
                receipts: receipts.unwrap_or_default(),
                reason,
                revert_id: None,
                total_gas,
                total_fee,
            }),
            ClientTransactionStatus::SqueezedOut { reason } => {
                TxStatus::SqueezedOut(SqueezedOut { reason })
            }
        }
    }
}

#[cfg(feature = "std")]
impl From<TransactionExecutionStatus> for TxStatus {
    fn from(value: TransactionExecutionStatus) -> Self {
        match value.result {
            TransactionExecutionResult::Success {
                receipts,
                total_gas,
                total_fee,
                ..
            } => Self::Success(Success {
                receipts,
                total_gas,
                total_fee,
            }),
            TransactionExecutionResult::Failed {
                result,
                receipts,
                total_gas,
                total_fee,
                ..
            } => {
                let revert_id = result.and_then(|result| match result {
                    ProgramState::Revert(revert_id) => Some(revert_id),
                    _ => None,
                });
                let reason = TransactionExecutionResult::reason(&receipts, &result);

                Self::Failure(Failure {
                    receipts,
                    reason,
                    revert_id,
                    total_gas,
                    total_fee,
                })
            }
        }
    }
}
