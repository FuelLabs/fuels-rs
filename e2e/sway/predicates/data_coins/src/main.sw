predicate;

use std::{inputs::*, outputs::*};

struct DataCoinConfig {
    num_participants: u64,
}

fn main(_unused: DataCoinConfig) -> bool {
    // turn predicate coin into datacoin and pay with wallet
    let input_count = input_count(); // predicate and wallet coin
    let output_count = output_count(); // datacoin and wallet change
    match (input_count, output_count) {
        (2, 2) => {
            // Check:
            // - predicate coin amount is equal to datacoin amount
            // - datacoin data can be decoded into `DataCoinConfig`
            // - `DataCoinConfig` num_praticipants == 1 (initial config creation)
            let predicate_coin_amount = input_amount(0).unwrap();
            let data_coin_amount = output_amount(0).unwrap();

            if predicate_coin_amount != data_coin_amount {
                return false;
            }

            let datacoin_config = output_data_coin_data::<DataCoinConfig>(0).unwrap();

            if datacoin_config.num_participants != 1 {
                return false;
            }

            return true;
        },
        _ => return false,
    }
}
