predicate;

use std::tx::tx_witness_data;

fn main(witness_index: u64, witness_index2: u64) -> bool {
    let witness: u8 = tx_witness_data(witness_index).unwrap();
    let witness2: u64 = tx_witness_data(witness_index2).unwrap();

    witness == 64 && witness2 == 4096
}
