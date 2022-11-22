use fuel_tx::Receipt;

/// [`VMCallResponse`] is a struct that is returned by a call to the contract or script. Its value
/// field holds the decoded typed value returned by the contract's method. The other field holds all
/// the receipts returned by the call.
///
/// The name is `VMCallResponse` instead of `CallResponse` because it would be ambiguous with the
/// `CALL` opcode.
#[derive(Debug)]
// ANCHOR: fuel_call_response
pub struct FuelCallResponse<D> {
    pub value: D,
    pub receipts: Vec<Receipt>,
    pub gas_used: u64,
}
// ANCHOR_END: fuel_call_response

impl<D> FuelCallResponse<D> {
    /// Get the gas used from ScriptResult receipt
    fn get_gas_used(receipts: &[Receipt]) -> u64 {
        receipts
            .iter()
            .rfind(|r| matches!(r, Receipt::ScriptResult { .. }))
            .expect("could not retrieve ScriptResult")
            .gas_used()
            .expect("could not retrieve gas used from ScriptResult")
    }

    pub fn new(value: D, receipts: Vec<Receipt>) -> Self {
        Self {
            value,
            gas_used: Self::get_gas_used(&receipts),
            receipts,
        }
    }
}
