use fuel_abi_types::error_codes::{
    FAILED_ASSERT_EQ_SIGNAL, FAILED_ASSERT_SIGNAL, FAILED_REQUIRE_SIGNAL,
    FAILED_SEND_MESSAGE_SIGNAL, FAILED_TRANSFER_TO_ADDRESS_SIGNAL,
};
use fuel_tx::Receipt;

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
    SqueezedOut,
    Revert {
        receipts: Vec<Receipt>,
        reason: String,
        id: u64,
    },
}

impl TxStatus {
    pub fn check(&self, log_decoder: Option<&LogDecoder>) -> Result<()> {
        let Self::Revert {
            receipts,
            reason,
            id,
        } = &self
        else {
            return Ok(());
        };

        let reason = match (*id, log_decoder) {
            (FAILED_REQUIRE_SIGNAL, Some(log_decoder)) => log_decoder
                .decode_last_log(receipts)
                .unwrap_or_else(|err| format!("failed to decode log from require revert: {err}")),
            (FAILED_ASSERT_EQ_SIGNAL, Some(log_decoder)) => {
                match log_decoder.decode_last_two_logs(receipts) {
                    Ok((lhs, rhs)) => format!(
                        "assertion failed: `(left == right)`\n left: `{lhs:?}`\n right: `{rhs:?}`"
                    ),
                    Err(err) => format!("failed to decode log from assert_eq revert: {err}"),
                }
            }
            (FAILED_ASSERT_SIGNAL, _) => "assertion failed.".into(),
            (FAILED_SEND_MESSAGE_SIGNAL, _) => "failed to send message.".into(),
            (FAILED_TRANSFER_TO_ADDRESS_SIGNAL, _) => "failed transfer to address.".into(),
            _ => reason.clone(),
        };

        Err(Error::RevertTransactionError {
            reason,
            revert_id: *id,
            receipts: receipts.clone(),
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
