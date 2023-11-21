use chrono::{DateTime, Utc};
use fuel_abi_types::error_codes::{
    FAILED_ASSERT_EQ_SIGNAL, FAILED_ASSERT_SIGNAL, FAILED_REQUIRE_SIGNAL,
    FAILED_SEND_MESSAGE_SIGNAL, FAILED_TRANSFER_TO_ADDRESS_SIGNAL,
};
use fuel_core_client::client::types::primitives::BlockId;
#[cfg(feature = "std")]
use fuel_core_client::client::types::TransactionStatus as ClientTransactionStatus;
use fuel_tx::Receipt;
use fuel_types::Bytes32;
#[cfg(feature = "std")]
use fuel_vm::state::ProgramState;
use std::str::FromStr;
use tai64::Tai64;

use crate::{
    codec::LogDecoder,
    error,
    types::errors::{Error, Result},
};

#[derive(Debug, Clone)]
pub enum TxStatus {
    Success {
        block_id: Bytes32,
        time: DateTime<Utc>,
        program_state: Option<ProgramState>,
    },
    Submitted {
        submitted_at: DateTime<Utc>,
    },
    SqueezedOut {
        reason: String,
    },
    Revert {
        block_id: Bytes32,
        reason: String,
        revert_id: u64,
        time: DateTime<Utc>,
    },
}

impl TxStatus {
    pub fn check(&self, receipts: &[Receipt], log_decoder: Option<&LogDecoder>) -> Result<()> {
        match self {
            Self::Revert {
                reason,
                revert_id: id,
                ..
            } => Self::map_revert_error(receipts, reason, *id, log_decoder),
            Self::Success { .. } => Ok(()),
            Self::Submitted { .. } => Err(error!(
                InvalidData,
                "Calling .check on a Submitted transaction"
            )),
            Self::SqueezedOut { .. } => Err(error!(
                InvalidData,
                "Calling .check on a SqueezedOUt transaction"
            )),
        }
    }

    pub fn is_revert_or_success(&self) -> Result<()> {
        match self {
            Self::Success { .. } => Ok(()),
            Self::Revert { .. } => Ok(()),
            Self::SqueezedOut { reason } => Err(Error::SqueezedOutTransactionError(reason.clone())),
            Self::Submitted { .. } => Err(Error::ProviderError(
                "Transaction is only in submitted state".to_string(),
            )),
        }
    }

    fn map_revert_error(
        receipts: &[Receipt],
        reason: &str,
        id: u64,
        log_decoder: Option<&LogDecoder>,
    ) -> Result<()> {
        let reason = match (id, log_decoder) {
            (FAILED_REQUIRE_SIGNAL, Some(log_decoder)) => log_decoder
                .decode_last_log(receipts)
                .unwrap_or_else(|err| format!("failed to decode log from require revert: {err}")),
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
            (FAILED_ASSERT_SIGNAL, _) => "assertion failed.".into(),
            (FAILED_SEND_MESSAGE_SIGNAL, _) => "failed to send message.".into(),
            (FAILED_TRANSFER_TO_ADDRESS_SIGNAL, _) => "failed transfer to address.".into(),
            _ => reason.to_string(),
        };

        Err(Error::RevertTransactionError {
            reason,
            revert_id: id,
            receipts: receipts.to_vec(),
        })
    }
}
#[cfg(feature = "std")]
impl From<ClientTransactionStatus> for TxStatus {
    fn from(client_status: ClientTransactionStatus) -> Self {
        let convert_timestamp = |timestamp: Tai64| {
            DateTime::from_timestamp(timestamp.to_unix(), 0)
                .ok_or(error!(InvalidData, "Timestamp should be valid UTC"))
                .expect("Timestamp should be valid UTC")
        };
        let convert_block_id = |block_id_string: String| {
            BlockId::from_str(block_id_string.as_str()).expect("Block id should be valid")
        };
        match client_status {
            ClientTransactionStatus::SqueezedOut { reason } => TxStatus::SqueezedOut { reason },
            ClientTransactionStatus::Submitted { submitted_at } => TxStatus::Submitted {
                submitted_at: convert_timestamp(submitted_at),
            },
            ClientTransactionStatus::Success {
                block_id,
                time,
                program_state,
            } => TxStatus::Success {
                block_id: convert_block_id(block_id),
                time: convert_timestamp(time),
                program_state,
            },
            ClientTransactionStatus::Failure {
                block_id,
                time,
                reason,
                program_state,
            } => {
                let revert_id = program_state
                    .and_then(|state| match state {
                        ProgramState::Revert(revert_id) => Some(revert_id),
                        _ => None,
                    })
                    .expect("Transaction failed without a `revert_id`");
                TxStatus::Revert {
                    time: convert_timestamp(time),
                    reason,
                    revert_id,
                    block_id: convert_block_id(block_id),
                }
            }
        }
    }
}
