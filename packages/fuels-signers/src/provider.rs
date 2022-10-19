use std::io;

#[cfg(feature = "fuel-core")]
use fuel_core::service::{Config, FuelService};

use fuel_gql_client::{
    client::{
        schema::{
            balance::Balance, block::Block, chain::ChainInfo, coin::Coin, contract::Contract,
            contract::ContractBalance, message::Message, node_info::NodeInfo, resource::Resource,
        },
        types::{TransactionResponse, TransactionStatus},
        FuelClient, PageDirection, PaginationRequest,
    },
    fuel_tx::{Receipt, Transaction, TransactionFee, UtxoId},
    fuel_types::{AssetId, ContractId},
};
use fuels_core::constants::{DEFAULT_GAS_ESTIMATION_TOLERANCE, MAX_GAS_PER_TX};
use std::{collections::HashMap, str::FromStr};
use thiserror::Error;

use fuels_types::bech32::{Bech32Address, Bech32ContractId};
use fuels_types::errors::Error;

#[derive(Debug)]
pub struct TransactionCost {
    pub min_gas_price: u64,
    pub gas_price: u64,
    pub gas_used: u64,
    pub metered_bytes_size: u64,
    pub total_fee: u64,
}

#[derive(Debug, Error)]
pub enum ProviderError {
    // Every IO error in the context of Provider comes from the gql client
    #[error(transparent)]
    ClientRequestError(#[from] io::Error),
}

impl From<ProviderError> for Error {
    fn from(e: ProviderError) -> Self {
        Error::ProviderError(e.to_string())
    }
}

/// Encapsulates common client operations in the SDK.
/// Note that you may also use `client`, which is an instance
/// of `FuelClient`, directly, which provides a broader API.
#[derive(Debug, Clone)]
pub struct Provider {
    pub client: FuelClient,
}

impl Provider {
    pub fn new(client: FuelClient) -> Self {
        Self { client }
    }

    /// Sends a transaction to the underlying Provider's client.
    /// # Examples
    ///
    /// ## Sending a transaction
    ///
    /// ```
    /// use fuels::tx::Transaction;
    /// use fuels::prelude::*;
    /// async fn foo() -> Result<(), Box<dyn std::error::Error>> {
    ///   // Setup local test node
    ///   let (provider, _) = setup_test_provider(vec![], vec![], None).await;
    ///   let tx = Transaction::default();
    ///
    ///   let receipts = provider.send_transaction(&tx).await?;
    ///   dbg!(receipts);
    ///
    ///   Ok(())
    /// }
    /// ```
    pub async fn send_transaction(&self, tx: &Transaction) -> Result<Vec<Receipt>, Error> {
        let tolerance = 0.0;
        let TransactionCost {
            gas_used,
            min_gas_price,
            ..
        } = self.estimate_transaction_cost(tx, Some(tolerance)).await?;

        if gas_used > tx.gas_limit() {
            return Err(Error::ProviderError(format!(
                "gas_limit({}) is lower than the estimated gas_used({})",
                tx.gas_limit(),
                gas_used
            )));
        } else if min_gas_price > tx.gas_price() {
            return Err(Error::ProviderError(format!(
                "gas_price({}) is lower than the required min_gas_price({})",
                tx.gas_price(),
                min_gas_price
            )));
        }

        let (status, receipts) = self.submit_with_feedback(tx).await?;

        if let TransactionStatus::Failure { reason, .. } = status {
            Err(Error::RevertTransactionError(reason, receipts))
        } else {
            Ok(receipts)
        }
    }

    async fn submit_with_feedback(
        &self,
        tx: &Transaction,
    ) -> Result<(TransactionStatus, Vec<Receipt>), ProviderError> {
        let tx_id = self.client.submit(tx).await?.0.to_string();
        let receipts = self.client.receipts(&tx_id).await?;
        let status = self.client.transaction_status(&tx_id).await?;

        Ok((status, receipts))
    }

    #[cfg(feature = "fuel-core")]
    /// Launches a local `fuel-core` network based on provided config.
    pub async fn launch(config: Config) -> Result<FuelClient, Error> {
        let srv = FuelService::new_node(config).await.unwrap();
        Ok(FuelClient::from(srv.bound_address))
    }

    /// Connects to an existing node at the given address.
    /// # Examples
    ///
    /// ## Connect to a node
    /// ```
    /// async fn connect_to_fuel_node() {
    ///     use fuels::prelude::*;
    ///
    ///     // This is the address of a running node.
    ///     let server_address = "127.0.0.1:4000";
    ///
    ///     // Create the provider using the client.
    ///     let provider = Provider::connect(server_address).await.unwrap();
    ///
    ///     // Create the wallet.
    ///     let _wallet = WalletUnlocked::new_random(Some(provider));
    /// }
    /// ```
    pub async fn connect(url: impl AsRef<str>) -> Result<Provider, Error> {
        let client = FuelClient::new(url)?;
        Ok(Provider::new(client))
    }

    pub async fn chain_info(&self) -> Result<ChainInfo, ProviderError> {
        Ok(self.client.chain_info().await?)
    }

    pub async fn node_info(&self) -> Result<NodeInfo, ProviderError> {
        Ok(self.client.node_info().await?)
    }

    pub async fn dry_run(&self, tx: &Transaction) -> Result<Vec<Receipt>, ProviderError> {
        Ok(self.client.dry_run(tx).await?)
    }

    pub async fn dry_run_no_validation(
        &self,
        tx: &Transaction,
    ) -> Result<Vec<Receipt>, ProviderError> {
        Ok(self.client.dry_run_opt(tx, Some(false)).await?)
    }

    pub async fn get_coin(&self, id: &UtxoId) -> Result<Option<Coin>, ProviderError> {
        let hex_id = format!("{:#x}", id);
        self.client.coin(&hex_id).await.map_err(Into::into)
    }

    /// Gets all coins owned by address `from`, with asset ID `asset_id`, *even spent ones*. This
    /// returns actual coins (UTXOs).
    pub async fn get_coins(
        &self,
        from: &Bech32Address,
        asset_id: &AssetId,
    ) -> Result<Vec<Coin>, ProviderError> {
        let mut coins: Vec<Coin> = vec![];

        let mut cursor = None;

        loop {
            let res = self
                .client
                .coins(
                    &from.hash().to_string(),
                    Some(&asset_id.to_string()),
                    PaginationRequest {
                        cursor: cursor.clone(),
                        results: 100,
                        direction: PageDirection::Forward,
                    },
                )
                .await?;

            if res.results.is_empty() {
                break;
            }
            coins.extend(res.results);
            cursor = res.cursor;
        }

        Ok(coins)
    }

    /// Get some spendable coins of asset `asset_id` for address `from` that add up at least to
    /// amount `amount`. The returned coins (UTXOs) are actual coins that can be spent. The number
    /// of coins (UXTOs) is optimized to prevent dust accumulation.
    pub async fn get_spendable_coins(
        &self,
        from: &Bech32Address,
        asset_id: &AssetId,
        amount: u64,
    ) -> Result<Vec<Coin>, ProviderError> {
        let res = self
            .client
            .resources_to_spend(
                &from.hash().to_string(),
                vec![(format!("{:#x}", asset_id).as_str(), amount, None)],
                None,
            )
            .await?;

        let coins = res
            .into_iter()
            .flatten()
            .filter_map(|r| match r {
                Resource::Coin(c) => Some(c),
                _ => None,
            })
            .collect();

        Ok(coins)
    }

    /// Get the balance of all spendable coins `asset_id` for address `address`. This is different
    /// from getting coins because we are just returning a number (the sum of UTXOs amount) instead
    /// of the UTXOs.
    pub async fn get_asset_balance(
        &self,
        address: &Bech32Address,
        asset_id: &AssetId,
    ) -> Result<u64, ProviderError> {
        self.client
            .balance(&address.hash().to_string(), Some(&*asset_id.to_string()))
            .await
            .map_err(Into::into)
    }

    /// Get the balance of all spendable coins `asset_id` for contract with id `contract_id`.
    pub async fn get_contract_asset_balance(
        &self,
        contract_id: &Bech32ContractId,
        asset_id: &AssetId,
    ) -> Result<u64, ProviderError> {
        self.client
            .contract_balance(&contract_id.hash().to_string(), Some(&asset_id.to_string()))
            .await
            .map_err(Into::into)
    }

    /// Get all the spendable balances of all assets for address `address`. This is different from
    /// getting the coins because we are only returning the numbers (the sum of UTXOs coins amount
    /// for each asset id) and not the UTXOs coins themselves
    pub async fn get_balances(
        &self,
        address: &Bech32Address,
    ) -> Result<HashMap<AssetId, u64>, ProviderError> {
        // We don't paginate results because there are likely at most ~100 different assets in one
        // wallet
        let pagination = PaginationRequest {
            cursor: None,
            results: 9999,
            direction: PageDirection::Forward,
        };
        let balances_vec = self
            .client
            .balances(&address.hash().to_string(), pagination)
            .await?
            .results;
        let balances = balances_vec
            .into_iter()
            .map(
                |Balance {
                     owner: _,
                     amount,
                     asset_id,
                 }| {
                    (
                        AssetId::from_str(&asset_id.to_string()).unwrap(),
                        amount.try_into().unwrap(),
                    )
                },
            )
            .collect();
        Ok(balances)
    }

    /// Get all balances of all assets for the contract with id `contract_id`.
    pub async fn get_contract_balances(
        &self,
        contract_id: &Bech32ContractId,
    ) -> Result<HashMap<AssetId, u64>, ProviderError> {
        // We don't paginate results because there are likely at most ~100 different assets in one
        // wallet
        let pagination = PaginationRequest {
            cursor: None,
            results: 9999,
            direction: PageDirection::Forward,
        };

        let balances_vec = self
            .client
            .contract_balances(&contract_id.hash().to_string(), pagination)
            .await?
            .results;
        let balances = balances_vec
            .into_iter()
            .map(
                |ContractBalance {
                     contract: _,
                     amount,
                     asset_id,
                 }| {
                    (
                        AssetId::from_str(&asset_id.to_string()).unwrap(),
                        amount.try_into().unwrap(),
                    )
                },
            )
            .collect();
        Ok(balances)
    }

    // Get a Contract with fields: id, bytecode (as hex string) and salt, from the client
    pub async fn get_contract(
        &self,
        id: &Bech32ContractId,
    ) -> Result<Option<Contract>, ProviderError> {
        let hex_id = format!("{:#x}", ContractId::from(id));
        self.client.contract(&hex_id).await.map_err(Into::into)
    }

    pub async fn get_transaction(
        &self,
        tx_id: &str,
    ) -> Result<Option<TransactionResponse>, ProviderError> {
        self.client.transaction(tx_id).await.map_err(Into::into)
    }

    pub async fn get_transactions(&self) -> Result<Vec<TransactionResponse>, ProviderError> {
        let mut transaction_responses: Vec<TransactionResponse> = vec![];

        let mut cursor = None;

        loop {
            let res = self
                .client
                .transactions(PaginationRequest {
                    cursor: cursor.clone(),
                    results: 100,
                    direction: PageDirection::Forward,
                })
                .await?;

            if res.results.is_empty() {
                break;
            }
            transaction_responses.extend(res.results);
            cursor = res.cursor;
        }

        Ok(transaction_responses)
    }

    pub async fn get_transactions_by_owner(
        &self,
        owner: &Bech32Address,
    ) -> Result<Vec<TransactionResponse>, ProviderError> {
        let mut transaction_responses: Vec<TransactionResponse> = vec![];

        let mut cursor = None;

        loop {
            let res = self
                .client
                .transactions_by_owner(
                    &owner.hash().to_string(),
                    PaginationRequest {
                        cursor: cursor.clone(),
                        results: 100,
                        direction: PageDirection::Forward,
                    },
                )
                .await?;

            if res.results.is_empty() {
                break;
            }
            transaction_responses.extend(res.results);
            cursor = res.cursor;
        }

        Ok(transaction_responses)
    }

    pub async fn latest_block_height(&self) -> Result<u64, ProviderError> {
        Ok(self.client.chain_info().await?.latest_block.height.0)
    }

    pub async fn produce_blocks(&self, amount: u64) -> io::Result<u64> {
        self.client.produce_blocks(amount).await
    }

    pub async fn estimate_transaction_cost(
        &self,
        tx: &Transaction,
        tolerance: Option<f64>,
    ) -> Result<TransactionCost, Error> {
        let NodeInfo { min_gas_price, .. } = self.node_info().await?;

        let tolerance = tolerance.unwrap_or(DEFAULT_GAS_ESTIMATION_TOLERANCE);
        let mut dry_run_tx = Self::generate_dry_run_tx(tx);
        let consensus_parameters = self.chain_info().await?.consensus_parameters;
        let gas_used = self
            .get_gas_used_with_tolerance(&dry_run_tx, tolerance)
            .await?;
        let gas_price = std::cmp::max(tx.gas_price(), min_gas_price.0);

        // Update the dry_run_tx with estimated gas_used and correct gas price to calculate the total_fee
        dry_run_tx.set_gas_price(gas_price);
        dry_run_tx.set_gas_limit(gas_used);

        let transaction_fee =
            TransactionFee::checked_from_tx(&consensus_parameters.into(), &dry_run_tx)
                .expect("Error calculating TransactionFee");

        Ok(TransactionCost {
            min_gas_price: min_gas_price.0,
            gas_price,
            gas_used,
            metered_bytes_size: tx.metered_bytes_size() as u64,
            total_fee: transaction_fee.total(),
        })
    }

    // Remove limits from an existing Transaction to get an accurate gas estimation
    fn generate_dry_run_tx(tx: &Transaction) -> Transaction {
        let mut dry_run_tx = tx.clone();
        // Simulate the contract call with MAX_GAS_PER_TX to get the complete gas_used
        dry_run_tx.set_gas_limit(MAX_GAS_PER_TX);
        dry_run_tx.set_gas_price(0);
        dry_run_tx
    }

    // Increase estimated gas by the provided tolerance
    async fn get_gas_used_with_tolerance(
        &self,
        tx: &Transaction,
        tolerance: f64,
    ) -> Result<u64, ProviderError> {
        let gas_used = self.get_gas_used(&self.dry_run_no_validation(tx).await?);
        Ok((gas_used as f64 * (1.0 + tolerance)) as u64)
    }

    fn get_gas_used(&self, receipts: &[Receipt]) -> u64 {
        receipts
            .iter()
            .rfind(|r| matches!(r, Receipt::ScriptResult { .. }))
            .map(|script_result| {
                script_result
                    .gas_used()
                    .expect("could not retrieve gas used from ScriptResult")
            })
            .unwrap_or(0)
    }

    pub async fn get_messages(&self, from: &Bech32Address) -> Result<Vec<Message>, ProviderError> {
        let pagination = PaginationRequest {
            cursor: None,
            results: 100,
            direction: PageDirection::Forward,
        };
        let res = self
            .client
            .messages(Some(&from.hash().to_string()), pagination)
            .await?;
        Ok(res.results)
    }

    pub async fn get_block(&self, block_id: &str) -> Result<Option<Block>, ProviderError> {
        self.client.block(block_id).await.map_err(Into::into)
    }

    pub async fn get_blocks(&self) -> Result<Vec<Block>, ProviderError> {
        let res = self
            .client
            .blocks(PaginationRequest {
                cursor: None,
                results: 999,
                direction: PageDirection::Forward,
            })
            .await?;
        Ok(res.results)
    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "test-helpers")]
    use fuel_core::model::Coin;

    use fuel_gql_client::client::types::TransactionStatus;
    use fuel_gql_client::fuel_tx::UtxoId;
    use fuels::prelude::*;

    async fn setup_provider_api_test() -> (
        WalletUnlocked,
        (Vec<(UtxoId, Coin)>, Vec<AssetId>),
        Provider,
    ) {
        let mut wallet = WalletUnlocked::new_random(None);
        let (coins, asset_ids) = setup_multiple_assets_coins(wallet.address(), 2, 4, 8);
        let (provider, _) = setup_test_provider(coins.clone(), vec![], None).await;
        wallet.set_provider(provider.clone());

        (wallet, (coins, asset_ids), provider)
    }

    #[tokio::test]
    async fn test_coin_api() -> Result<(), Error> {
        let (_, (coins, _), provider) = setup_provider_api_test().await;

        let (coin_id, _) = &coins[0];
        let hex_coin_id = format!("{:#x}", coin_id);

        let expected_coin = provider
            .get_coin(coin_id)
            .await?
            .expect("could not find coin with provided id");

        assert_eq!(hex_coin_id, expected_coin.utxo_id.0.to_string());
        Ok(())
    }

    #[tokio::test]
    async fn test_coins_api() -> Result<(), Error> {
        use std::collections::HashSet;

        let (wallet, (coins, asset_ids), provider) = setup_provider_api_test().await;
        let asset_id = &asset_ids[0];
        let utxo_ids_of_coins_with_asset_id: HashSet<String> = coins
            .iter()
            .filter(|c| c.1.asset_id == *asset_id)
            .map(|c| format!("{:#x}", c.0))
            .collect();

        let expected_coins = provider.get_coins(wallet.address(), asset_id).await?;

        assert_eq!(expected_coins.len(), utxo_ids_of_coins_with_asset_id.len());
        assert!(expected_coins
            .iter()
            .all(|ec| utxo_ids_of_coins_with_asset_id.contains(&ec.utxo_id.0.to_string())));

        Ok(())
    }

    #[tokio::test]
    async fn test_spendable_coins_api() -> Result<(), Error> {
        use std::collections::HashSet;

        let (wallet, (coins, asset_ids), provider) = setup_provider_api_test().await;
        let asset_id = &asset_ids[0];
        let amount = 18;
        let utxo_ids_of_coins_with_asset_id: HashSet<String> = coins
            .iter()
            .filter(|c| c.1.asset_id == *asset_id)
            .map(|c| format!("{:#x}", c.0))
            .collect();

        let expected_coins = provider
            .get_spendable_coins(wallet.address(), asset_id, amount)
            .await?;

        assert!(expected_coins.iter().map(|ec| ec.amount.0).sum::<u64>() > amount);
        assert!(expected_coins
            .iter()
            .all(|ec| utxo_ids_of_coins_with_asset_id.contains(&ec.utxo_id.0.to_string())));

        Ok(())
    }

    #[tokio::test]
    async fn test_asset_balance_api() -> Result<(), Error> {
        let (wallet, (coins, asset_ids), provider) = setup_provider_api_test().await;
        let asset_id = &asset_ids[0];
        let balance_of_coins_with_asset_id: u64 = coins
            .iter()
            .filter(|c| c.1.asset_id == *asset_id)
            .map(|c| c.1.amount)
            .sum();

        let expected_balance = provider
            .get_asset_balance(wallet.address(), asset_id)
            .await?;

        assert_eq!(balance_of_coins_with_asset_id, expected_balance);

        Ok(())
    }

    #[tokio::test]
    async fn test_contract_asset_balance_api() -> Result<(), Error> {
        let (wallet, (_, asset_ids), provider) = setup_provider_api_test().await;
        let asset_id = &asset_ids[0];

        let contract_id = Contract::deploy(
            "../fuels/tests/contracts/contract_test/out/debug/contract_test.bin",
            &wallet,
            TxParameters::default(),
            StorageConfiguration::default(),
        )
        .await?;

        let amount = 18;
        let _receipts = wallet
            .force_transfer_to_contract(&contract_id, amount, *asset_id, TxParameters::default())
            .await?;

        let expected_contract_balance = provider
            .get_contract_asset_balance(&contract_id, asset_id)
            .await?;

        assert_eq!(expected_contract_balance, amount);

        Ok(())
    }

    #[tokio::test]
    async fn test_balances_api() -> Result<(), Error> {
        let (wallet, (coins, asset_ids), provider) = setup_provider_api_test().await;
        let asset_id = &asset_ids[0];
        let wallet_balance_asset_id: u64 = coins
            .iter()
            .filter(|c| c.1.asset_id == *asset_id)
            .map(|c| c.1.amount)
            .sum();

        let wallet_balances = provider.get_balances(wallet.address()).await?;
        let expected_asset_balance = wallet_balances
            .get(asset_id)
            .expect("could not get balance for asset id");

        assert_eq!(*expected_asset_balance, wallet_balance_asset_id);

        Ok(())
    }

    #[tokio::test]
    async fn test_contract_balances_api() -> Result<(), Error> {
        let (wallet, (_, asset_ids), provider) = setup_provider_api_test().await;
        let asset_id = &asset_ids[0];

        let contract_id = Contract::deploy(
            "../fuels/tests/contracts/contract_test/out/debug/contract_test.bin",
            &wallet,
            TxParameters::default(),
            StorageConfiguration::default(),
        )
        .await?;

        let amount = 18;
        let _receipts = wallet
            .force_transfer_to_contract(&contract_id, amount, *asset_id, TxParameters::default())
            .await?;

        let contract_balances = provider.get_contract_balances(&contract_id).await?;

        let expected_asset_balance = contract_balances
            .get(asset_id)
            .expect("could not get balance for asset id");
        assert_eq!(*expected_asset_balance, amount);

        Ok(())
    }

    #[tokio::test]
    async fn test_get_contract_api() -> Result<(), Error> {
        let (wallet, (_, _), provider) = setup_provider_api_test().await;

        let contract_id = Contract::deploy(
            "../fuels/tests/contracts/contract_test/out/debug/contract_test.bin",
            &wallet,
            TxParameters::default(),
            StorageConfiguration::default(),
        )
        .await?;
        let hex_contract_id = format!("{:#x}", ContractId::from(&contract_id));

        let expected_contract = provider
            .get_contract(&contract_id)
            .await?
            .expect("could not find contract with specified id");

        assert_eq!(hex_contract_id, expected_contract.id.to_string());

        Ok(())
    }

    #[tokio::test]
    async fn test_transaction_api() -> Result<(), Error> {
        let (wallet, (_, _), provider) = setup_provider_api_test().await;

        let wallet2 = WalletUnlocked::new_random(Some(provider.clone()));

        let gas_price = 1;
        let gas_limit = 500_000;
        let maturity = 0;
        let tx_params = TxParameters {
            gas_price,
            gas_limit,
            maturity,
        };

        let (tx_id, _receipts) = wallet
            .transfer(wallet2.address(), 1, Default::default(), tx_params)
            .await?;

        let expected_tresponse = provider
            .get_transaction(&tx_id)
            .await?
            .expect("could not find transaction with specified id");

        assert_eq!(expected_tresponse.transaction.gas_limit(), gas_limit);
        assert_eq!(expected_tresponse.transaction.gas_price(), gas_price);
        assert_eq!(expected_tresponse.transaction.maturity(), maturity);

        Ok(())
    }

    #[tokio::test]
    async fn test_transactions_api() -> Result<(), Error> {
        let (wallet, (_, _), provider) = setup_provider_api_test().await;

        let wallet2 = WalletUnlocked::new_random(Some(provider.clone()));

        // Make two transactions
        let (_tx_id1, _receipts) = wallet
            .transfer(wallet2.address(), 1, Default::default(), Default::default())
            .await?;
        let (_tx_id2, _receipts) = wallet
            .transfer(wallet2.address(), 1, Default::default(), Default::default())
            .await?;

        let expected_response = provider.get_transactions().await?;

        assert_eq!(expected_response.len(), 2);
        //TODO: check if I can test it in another way

        Ok(())
    }

    #[tokio::test]
    async fn test_transaction_by_owner_api() -> Result<(), Error> {
        let (wallet, (_, _), provider) = setup_provider_api_test().await;

        let wallet2 = WalletUnlocked::new_random(Some(provider.clone()));

        // Make two transactions
        let (_tx_id1, _receipts) = wallet
            .transfer(wallet2.address(), 1, Default::default(), Default::default())
            .await?;
        let (_tx_id2, _receipts) = wallet
            .transfer(wallet2.address(), 1, Default::default(), Default::default())
            .await?;

        let expected_response = provider.get_transactions_by_owner(wallet.address()).await?;

        assert_eq!(expected_response.len(), 2);
        //TODO: check if I can test it in another way

        Ok(())
    }

    #[tokio::test]
    async fn test_block_api() -> Result<(), Error> {
        let (wallet, (_, _), provider) = setup_provider_api_test().await;

        let wallet2 = WalletUnlocked::new_random(Some(provider.clone()));

        let (tx_id, _receipts) = wallet
            .transfer(wallet2.address(), 1, Default::default(), Default::default())
            .await?;

        let transaction_response = provider
            .get_transaction(&tx_id)
            .await?
            .expect("could not find transaction with specified id");

        if let TransactionStatus::Success { block_id, time, .. } = transaction_response.status {
            let expected_block = provider
                .get_block(&block_id)
                .await?
                .expect("could not find block with specified id");

            assert_eq!(block_id, expected_block.id.to_string());
            assert_eq!(expected_block.time, time);

            return Ok(());
        }

        Err(Error::ProviderError(
            "Transaction was not successfull".into(),
        ))
    }

    #[tokio::test]
    async fn test_blocks_api() -> Result<(), Error> {
        let (wallet, (_, _), provider) = setup_provider_api_test().await;

        let wallet2 = WalletUnlocked::new_random(Some(provider.clone()));

        // Make two transactions
        let (_tx_id1, _receipts) = wallet
            .transfer(wallet2.address(), 1, Default::default(), Default::default())
            .await?;
        let (_tx_id2, _receipts) = wallet
            .transfer(wallet2.address(), 1, Default::default(), Default::default())
            .await?;

        let expected_blocks = provider.get_blocks().await?;

        assert_eq!(expected_blocks.len(), 2);
        //TODO: check if I can test it in another way

        Ok(())
    }
}
