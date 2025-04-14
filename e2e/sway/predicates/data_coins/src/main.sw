predicate;

use std::{inputs::*, outputs::*};

struct DataCoinConfig {
    num_participants: u64,
}

fn main(_unused: DataCoinConfig) -> bool {
    let input_count = input_count();
    let output_count = output_count();
    match (input_count, output_count) {
        // transfer some amount from predicate coin
        // to data coin and return change to predicate
        (1, 2) => { //(predicate_coin, data_coin and predicate change)
            // Check:
            // - predicate coin amount is bigger than data_coin amount
            // - data_coin data can be decoded into `DataCoinConfig`
            // - `DataCoinConfig` `num_praticipants == 1` (initial config creation)
            let predicate_coin_amount = input_amount(0).unwrap();
            let data_coin_amount = output_amount(0).unwrap();

            if predicate_coin_amount < data_coin_amount {
                return false;
            }

            let datacoin_config = output_data_coin_data::<DataCoinConfig>(0).unwrap();

            if datacoin_config.num_participants != 1 {
                return false;
            }

            return true;
        },
        // update input data coin with new amount from predicate coin and incremented `num_participants`
        (2, 2) => { //(data_coin and predicate_coin, data_coin and predicate change)
            // Check:
            // - predicate coin amount is bigger than (output_data_coin_amount - input_data_coin_amount)
            // - data_coin data can be decoded into `DataCoinConfig`
            // - output `DataCoinConfig` `num_praticipants` is incremented by 1
            let input_data_coin_amount = input_amount(0).unwrap();
            let predicate_coin_amount = input_amount(1).unwrap();
            let output_data_coin_amount = output_amount(0).unwrap();

            if predicate_coin_amount < (output_data_coin_amount - input_data_coin_amount)
            {
                return false;
            }

            let input_num_participants = input_data_coin_data::<DataCoinConfig>(0).unwrap().num_participants;
            let output_num_participants = output_data_coin_data::<DataCoinConfig>(0).unwrap().num_participants;

            if (output_num_participants - input_num_participants) != 1 {
                return false;
            }

            return true;
        },
        _ => return false,
    }
}
