use std::fmt::Debug;

use fuel_tx::{Bytes32, Receipt};
use fuels_core::{
    codec::{LogDecoder, LogResult},
    traits::{Parameterize, Tokenizable},
    types::errors::Result,
};

/// [`CallResponse`] is a struct that is returned by a call to the contract or script. Its value
/// field holds the decoded typed value returned by the contract's method. The other field holds all
/// the receipts returned by the call.
#[derive(Debug)]
// ANCHOR: call_response
pub struct CallResponse<D> {
    pub value: D,
    pub receipts: Vec<Receipt>,
    pub gas_used: u64,
    pub total_fee: u64,
    pub log_decoder: LogDecoder,
    pub tx_id: Option<Bytes32>,
}
// ANCHOR_END: call_response

impl<D> CallResponse<D> {
    pub fn decode_logs(&self) -> LogResult {
        self.log_decoder.decode_logs(&self.receipts)
    }

    pub fn decode_logs_with_type<T: Tokenizable + Parameterize + 'static>(&self) -> Result<Vec<T>> {
        self.log_decoder.decode_logs_with_type::<T>(&self.receipts)
    }
}
