#[cfg(test)]
mod tests {
    use fuels::prelude::Error;

    #[tokio::test]
    async fn connect_to_fuel_node() {
        // TODO: enable this code when the tesnet is online and remove code in the documentation
        // TODO: the code is in providers/external-node.md
        // AXXNCHOR: connect_to_node
        use fuels::prelude::*;
        use std::net::{SocketAddr, ToSocketAddrs};

        // This is the testnet's address
        let server_address: SocketAddr = "node-beta-1.fuel.network/graphql:443"
            .to_socket_addrs()
            .expect("Unable to parse or lookup address")
            .next() // Take the first resolved address
            .unwrap();

        // Create the provider using the client.
        let provider = Provider::connect(server_address).await.unwrap();

        // Create the wallet.
        let _wallet = WalletUnlocked::new_random(Some(provider));
        // AXXNCHOR_END: connect_to_node

        // ANCHOR: local_node_address
        let _server_address: SocketAddr = "127.0.0.1:4000"
            .parse()
            .expect("Unable to parse socket address");
        // ANCHOR_END: local_node_address
    }

    #[tokio::test]
    async fn query_the_blockchain() -> Result<(), Error> {
        // ANCHOR: setup_test_blockchain
        use fuels::prelude::*;

        // Set up our test blockchain.

        // Create a random wallet (more on wallets later).
        // ANCHOR: setup_single_asset
        let wallet = WalletUnlocked::new_random(None);

        // How many coins in our wallet.
        let number_of_coins = 1;

        // The amount/value in each coin in our wallet.
        let amount_per_coin = 3;

        let coins = setup_single_asset_coins(
            wallet.address(),
            BASE_ASSET_ID,
            number_of_coins,
            amount_per_coin,
        );
        // ANCHOR_END: setup_single_asset

        let (provider, _) = setup_test_provider(coins.clone(), None).await;
        // ANCHOR_END: setup_test_blockchain

        // ANCHOR: get_coins
        let coins = provider.get_coins(wallet.address(), BASE_ASSET_ID).await?;
        assert_eq!(coins.len(), 1);
        // ANCHOR_END: get_coins

        // ANCHOR: get_spendable_coins
        let spendable_coins = provider
            .get_spendable_coins(wallet.address(), BASE_ASSET_ID, 1)
            .await?;
        assert_eq!(spendable_coins.len(), 1);
        // ANCHOR_END: get_spendable_coins

        // ANCHOR: get_balances
        let _balances = provider.get_balances(wallet.address()).await?;
        // ANCHOR_END: get_balances

        Ok(())
    }
}
