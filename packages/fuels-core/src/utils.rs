pub mod constants;
pub mod offsets;

use constants::{WITNESS_STATIC_SIZE, WORD_SIZE};
use fuel_tx::Witness;

use crate::{error, types::errors::Result};

pub fn checked_round_up_to_word_alignment(bytes_len: usize) -> Result<usize> {
    let lhs = bytes_len.checked_add(WORD_SIZE - 1).ok_or_else(|| {
        error!(
            Codec,
            "addition overflow while rounding up {bytes_len} bytes to word alignment"
        )
    })?;
    let rhs = lhs.checked_rem(WORD_SIZE).ok_or_else(|| {
        error!(
            Codec,
            "remainder overflow while rounding up {bytes_len} bytes to word alignment"
        )
    })?;
    lhs.checked_sub(rhs).ok_or_else(|| {
        error!(
            Codec,
            "subtraction overflow while rounding up {bytes_len} bytes to word alignment"
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

pub(crate) mod sealed {
    pub trait Sealed {}
}

#[cfg(test)]
pub(crate) fn to_named<'a, I: IntoIterator<Item = &'a crate::types::param_types::ParamType>>(
    param_types: I,
) -> Vec<(String, crate::types::param_types::ParamType)> {
    param_types
        .into_iter()
        .map(|pt| ("".to_string(), pt.clone()))
        .collect()
}
