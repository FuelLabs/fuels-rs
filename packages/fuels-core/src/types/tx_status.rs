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
pub struct Revert {
    pub reason: String,
    pub receipts: Vec<Receipt>,
    pub revert_id: u64,
    pub total_fee: u64,
    pub total_gas: u64,
}

#[derive(Debug, Clone)]
pub enum TxStatus {
    Success(Success),
    Submitted,
    SqueezedOut(SqueezedOut),
    Revert(Revert),
}

impl TxStatus {
    pub fn check(&self, log_decoder: Option<&LogDecoder>) -> Result<()> {
        match self {
            Self::SqueezedOut(SqueezedOut { reason }) => {
                Err(Error::Transaction(Reason::SqueezedOut(reason.clone())))
            }
            Self::Revert(Revert {
                receipts,
                reason,
                revert_id: id,
                ..
            }) => Err(Self::map_revert_error(receipts, reason, *id, log_decoder)),
            _ => Ok(()),
        }
    }

    pub fn take_success_checked(self, log_decoder: Option<&LogDecoder>) -> Result<Success> {
        match self {
            Self::SqueezedOut(SqueezedOut { reason }) => {
                Err(Error::Transaction(Reason::SqueezedOut(reason.clone())))
            }
            Self::Revert(Revert {
                receipts,
                reason,
                revert_id: id,
                ..
            }) => Err(Self::map_revert_error(&receipts, &reason, id, log_decoder)),
            Self::Submitted => Err(Error::Transaction(Reason::Other(
                "transactions was not yet included".to_owned(),
            ))),
            Self::Success(success) => Ok(success),
        }
    }

    pub fn total_gas(&self) -> u64 {
        match self {
            TxStatus::Success(Success { total_gas, .. })
            | TxStatus::Revert(Revert { total_gas, .. }) => *total_gas,
            _ => 0,
        }
    }

    pub fn total_fee(&self) -> u64 {
        match self {
            TxStatus::Success(Success { total_fee, .. })
            | TxStatus::Revert(Revert { total_fee, .. }) => *total_fee,
            _ => 0,
        }
    }

    fn map_revert_error(
        receipts: &[Receipt],
        reason: &str,
        id: u64,
        log_decoder: Option<&LogDecoder>,
    ) -> Error {
        let reason = match (id, log_decoder) {
            (FAILED_REQUIRE_SIGNAL, Some(log_decoder)) => log_decoder
                .decode_last_log(receipts)
                .unwrap_or_else(|err| format!("failed to decode log from require revert: {err}")),
            (REVERT_WITH_LOG_SIGNAL, Some(log_decoder)) => log_decoder
                .decode_last_log(receipts)
                .unwrap_or_else(|err| format!("failed to decode log from revert_with_log: {err}")),
            (FAILED_ASSERT_EQ_SIGNAL, Some(log_decoder)) => {
                match log_decoder.decode_last_two_logs(receipts) {
                    Ok((lhs, rhs)) => format!(
                        "assertion failed: `(left == right)`\n left: `{lhs:?}`\n right: `{rhs:?}`"
                    ),
                    Err(err) => {
                        format!("failed to decode log from assert_eq revert: {err}")
                    }
                }
            }
            (FAILED_ASSERT_NE_SIGNAL, Some(log_decoder)) => {
                match log_decoder.decode_last_two_logs(receipts) {
                    Ok((lhs, rhs)) => format!(
                        "assertion failed: `(left != right)`\n left: `{lhs:?}`\n right: `{rhs:?}`"
                    ),
                    Err(err) => {
                        format!("failed to decode log from assert_eq revert: {err}")
                    }
                }
            }
            (FAILED_ASSERT_SIGNAL, _) => "assertion failed".into(),
            (FAILED_SEND_MESSAGE_SIGNAL, _) => "failed to send message".into(),
            (FAILED_TRANSFER_TO_ADDRESS_SIGNAL, _) => "failed transfer to address".into(),
            _ => reason.to_string(),
        };

        Error::Transaction(Reason::Reverted {
            reason,
            revert_id: id,
            receipts: receipts.to_vec(),
        })
    }

    pub fn take_receipts_checked(self, log_decoder: Option<&LogDecoder>) -> Result<Vec<Receipt>> {
        self.check(log_decoder)?;
        Ok(self.take_receipts())
    }

    pub fn take_receipts(self) -> Vec<Receipt> {
        match self {
            TxStatus::Success(Success { receipts, .. })
            | TxStatus::Revert(Revert { receipts, .. }) => receipts,
            _ => vec![],
        }
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
            ClientTransactionStatus::Failure {
                reason,
                program_state,
                receipts,
                total_gas,
                total_fee,
                ..
            } => {
                let revert_id = program_state
                    .and_then(|state| match state {
                        ProgramState::Revert(revert_id) => Some(revert_id),
                        _ => None,
                    })
                    .expect("Transaction failed without a `revert_id`");
                TxStatus::Revert(Revert {
                    receipts,
                    reason,
                    revert_id,
                    total_gas,
                    total_fee,
                })
            }
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
                let revert_id = result
                    .and_then(|result| match result {
                        ProgramState::Revert(revert_id) => Some(revert_id),
                        _ => None,
                    })
                    .expect("Transaction failed without a `revert_id`");
                let reason = TransactionExecutionResult::reason(&receipts, &result);
                Self::Revert(Revert {
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
