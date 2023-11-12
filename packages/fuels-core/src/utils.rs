pub mod constants;
pub mod offsets;

use constants::WORD_SIZE;
pub const fn round_up_to_word_alignment(bytes_len: usize) -> usize {
    (bytes_len + (WORD_SIZE - 1)) - ((bytes_len + (WORD_SIZE - 1)) % WORD_SIZE)
}
