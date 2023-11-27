pub mod constants;
pub mod offsets;

use constants::{WITNESS_STATIC_SIZE, WORD_SIZE};
use fuel_tx::Witness;

pub const fn round_up_to_word_alignment(bytes_len: usize) -> usize {
    (bytes_len + (WORD_SIZE - 1)) - ((bytes_len + (WORD_SIZE - 1)) % WORD_SIZE)
}

pub(crate) fn calculate_witnesses_size<'a, I: IntoIterator<Item = &'a Witness>>(
    witnesses: I,
) -> usize {
    witnesses
        .into_iter()
        .map(|w| w.as_ref().len() + WITNESS_STATIC_SIZE)
        .sum()
}
