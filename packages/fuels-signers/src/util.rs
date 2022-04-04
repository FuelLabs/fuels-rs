#[allow(missing_docs)]
#[cfg(feature = "test-helpers")]
/// Testing utilities
pub mod test_helpers {
    use crate::provider::Provider;
    use crate::LocalWallet;
    use fuel_core::{
        chain_config::{ChainConfig, CoinConfig, StateConfig},
        model::coin::{Coin, CoinStatus},
        service::{Config, DbType, FuelService},
    };
    use fuel_crypto::Hasher;
    use fuel_gql_client::client::FuelClient;
    use fuel_tx::{Address, Bytes32, Bytes64, UtxoId};
    use fuels_core::constants::DEFAULT_INITIAL_BALANCE;
    use rand::{Fill, Rng};
    use secp256k1::{PublicKey, Secp256k1, SecretKey};
    use std::net::SocketAddr;

    pub async fn setup_test_provider_and_wallet() -> (Provider, LocalWallet) {
        //  We build only 1 coin with amount TEST_COIN_AMOUNT, empirically determined to be
        //  sufficient right now
        let (pk, coins) = setup_address_and_coins(1, DEFAULT_INITIAL_BALANCE);
        // Setup a provider and node with the given coins
        let (provider, _) = setup_test_provider(coins).await;

        let wallet = LocalWallet::new_from_private_key(pk, provider.clone()).unwrap();
        (provider, wallet)
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
                    asset_id: Default::default(),
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

    // Setup a test provider with the given coins. We return the SocketAddr so the launched node
    // client can be connected to more easily (even though it is often ignored).
    pub async fn setup_test_provider(coins: Vec<(UtxoId, Coin)>) -> (Provider, SocketAddr) {
        let coin_configs = coins
            .into_iter()
            .map(|(utxo_id, coin)| CoinConfig {
                tx_id: Some(*utxo_id.tx_id()),
                output_index: Some(utxo_id.output_index() as u64),
                block_created: Some(coin.block_created),
                maturity: Some(coin.maturity),
                owner: coin.owner,
                amount: coin.amount,
                asset_id: coin.asset_id,
            })
            .collect();

        // Setup node config with genesis coins and utxo_validation enabled
        let config = Config {
            chain_conf: ChainConfig {
                initial_state: Some(StateConfig {
                    coins: Some(coin_configs),
                    ..StateConfig::default()
                }),
                ..ChainConfig::local_testnet()
            },
            database_type: DbType::InMemory,
            utxo_validation: true,
            ..Config::local_node()
        };

        let srv = FuelService::new_node(config).await.unwrap();
        let client = FuelClient::from(srv.bound_address);

        (Provider::new(client), srv.bound_address)
    }
}
