pub mod constants;
pub mod offsets;

use crate::error;
use crate::types::errors::Result;
use constants::{WITNESS_STATIC_SIZE, WORD_SIZE};
use fuel_tx::Witness;

pub fn checked_round_up_to_word_alignment(bytes_len: usize) -> Result<usize> {
    let lhs = bytes_len.checked_add(WORD_SIZE - 1).ok_or_else(|| {
        error!(
            InvalidType,
            "Addition overflow while rounding up {bytes_len} bytes to word alignment"
        )
    })?;
    let rhs = lhs.checked_rem(WORD_SIZE).ok_or_else(|| {
        error!(
            InvalidType,
            "Remainder overflow while rounding up {bytes_len} bytes to word alignment"
        )
    })?;
    lhs.checked_sub(rhs).ok_or_else(|| {
        error!(
            InvalidType,
            "Substraction overflow while rounding up {bytes_len} bytes to word alignment"
        )
    })
}
pub(crate) fn calculate_witnesses_size<'a, I: IntoIterator<Item = &'a Witness>>(
    witnesses: I,
) -> usize {
    witnesses
        .into_iter()
        .map(|w| w.as_ref().len() + WITNESS_STATIC_SIZE)
        .sum()
}
