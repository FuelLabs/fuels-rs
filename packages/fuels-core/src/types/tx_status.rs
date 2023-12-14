use fuel_abi_types::error_codes::{
    FAILED_ASSERT_EQ_SIGNAL, FAILED_ASSERT_SIGNAL, FAILED_REQUIRE_SIGNAL,
    FAILED_SEND_MESSAGE_SIGNAL, FAILED_TRANSFER_TO_ADDRESS_SIGNAL,
};
#[cfg(feature = "std")]
use fuel_core_client::client::types::TransactionStatus as ClientTransactionStatus;
use fuel_tx::Receipt;
#[cfg(feature = "std")]
use fuel_vm::state::ProgramState;

use crate::{
    codec::LogDecoder,
    types::errors::{Error, Result},
};

#[derive(Debug, Clone)]
pub enum TxStatus {
    Success {
        receipts: Vec<Receipt>,
    },
    Submitted,
    SqueezedOut {
        reason: String,
    },
    Revert {
        receipts: Vec<Receipt>,
        reason: String,
        revert_id: u64,
    },
}

impl TxStatus {
    pub fn check(&self, log_decoder: Option<&LogDecoder>) -> Result<()> {
        match self {
            Self::SqueezedOut { reason } => Err(Error::SqueezedOutTransactionError(reason.clone())),
            Self::Revert {
                receipts,
                reason,
                revert_id: id,
            } => Self::map_revert_error(receipts, reason, *id, log_decoder),
            _ => Ok(()),
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

    pub fn take_receipts_checked(self, log_decoder: Option<&LogDecoder>) -> Result<Vec<Receipt>> {
        self.check(log_decoder)?;
        Ok(self.take_receipts())
    }

    pub fn take_receipts(self) -> Vec<Receipt> {
        match self {
            TxStatus::Success { receipts } | TxStatus::Revert { receipts, .. } => receipts,
            _ => vec![],
        }
    }
}

#[cfg(feature = "std")]
impl From<ClientTransactionStatus> for TxStatus {
    fn from(client_status: ClientTransactionStatus) -> Self {
        match client_status {
            ClientTransactionStatus::Submitted { .. } => TxStatus::Submitted {},
            ClientTransactionStatus::Success { receipts, .. } => TxStatus::Success { receipts },
            ClientTransactionStatus::Failure {
                reason,
                program_state,
                receipts,
                ..
            } => {
                let revert_id = program_state
                    .and_then(|state| match state {
                        ProgramState::Revert(revert_id) => Some(revert_id),
                        _ => None,
                    })
                    .expect("Transaction failed without a `revert_id`");
                TxStatus::Revert {
                    receipts,
                    reason,
                    revert_id,
                }
            }
            ClientTransactionStatus::SqueezedOut { reason } => TxStatus::SqueezedOut { reason },
        }
    }
}
