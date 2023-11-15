contract;

use std::{logging::log, outputs::output_count, tx::{tx_witness_data, tx_witnesses_count}};

abi TestContract {
    fn read_witness(witness_idx: u64, witness_idx2: u64) -> u8;
}

impl TestContract for Contract {
    // ANCHOR: contract_witness_log_method
    fn read_witness(witness_idx: u64, witness_idx2: u64) -> u8 {
        let witness: u8 = tx_witness_data(witness_idx);
        let witness2: u8 = tx_witness_data(witness_idx2);

        assert_eq(witness, witness2);

        log(output_count());
        log(tx_witnesses_count());

        witness
    }
    // ANCHOR_END: contract_witness_log_method
}
