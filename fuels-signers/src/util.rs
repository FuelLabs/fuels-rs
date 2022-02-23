#[allow(missing_docs)]
#[cfg(feature = "test-helpers")]
/// Testing utilities
pub mod test_helpers {
    use crate::provider::Provider;
    use fuel_core::service::{Config, FuelService};
    use fuel_core::{
        database::Database,
        model::coin::{Coin, CoinStatus},
    };
    use fuel_gql_client::client::FuelClient;
    use fuel_tx::crypto::Hasher;
    use fuel_tx::{Address, Bytes32, Bytes64, UtxoId};
    use fuel_vm::prelude::Storage;
    use rand::{Fill, Rng};
    use secp256k1::{PublicKey, Secp256k1, SecretKey};

    pub async fn setup_local_node(coins: Vec<(UtxoId, Coin)>) -> FuelClient {
        let mut db = Database::default();
        for (utxo_id, coin) in coins {
            Storage::<UtxoId, Coin>::insert(&mut db, &utxo_id, &coin).unwrap();
        }

        let srv = FuelService::from_database(db, Config::local_node())
            .await
            .unwrap();
        FuelClient::from(srv.bound_address)
    }

    pub fn setup_address_and_coins(
        num_of_coins: usize,
        amount: u64,
    ) -> (SecretKey, Vec<(UtxoId, Coin)>) {
        let mut rng = rand::thread_rng();

        let secret_seed = rng.gen::<[u8; 32]>();

        let secret =
            SecretKey::from_slice(&secret_seed).expect("Failed to generate random secret!");

        let secp = Secp256k1::new();

        let public = PublicKey::from_secret_key(&secp, &secret).serialize_uncompressed();
        let public = Bytes64::try_from(&public[1..]).unwrap();
        let hashed = Hasher::hash(public);

        let coins: Vec<(UtxoId, Coin)> = (1..=num_of_coins)
            .map(|_i| {
                let coin = Coin {
                    owner: Address::from(*hashed),
                    amount,
                    color: Default::default(),
                    maturity: Default::default(),
                    status: CoinStatus::Unspent,
                    block_created: Default::default(),
                };

                let mut r = Bytes32::zeroed();
                r.try_fill(&mut rng).unwrap();
                let utxo_id = UtxoId::new(r, 0);
                (utxo_id, coin)
            })
            .collect();

        (secret, coins)
    }

    pub async fn setup_test_provider(coins: Vec<(UtxoId, Coin)>) -> Provider {
        let mut db = Database::default();
        for (utxo_id, coin) in coins {
            Storage::<UtxoId, Coin>::insert(&mut db, &utxo_id, &coin).unwrap();
        }

        let srv = FuelService::from_database(db, Config::local_node())
            .await
            .unwrap();
        let client = FuelClient::from(srv.bound_address);

        Provider::new(client)
    }
}
