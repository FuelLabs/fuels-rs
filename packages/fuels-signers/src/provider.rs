use std::io;
use std::net::SocketAddr;

#[cfg(feature = "fuel-core")]
use fuel_core::service::{Config, FuelService};
use fuel_gql_client::{
    client::{
        schema::{balance::Balance, chain::ChainInfo, coin::Coin, contract::ContractBalance},
        types::{TransactionResponse, TransactionStatus},
        FuelClient, PageDirection, PaginatedResult, PaginationRequest,
    },
    fuel_tx::{ConsensusParameters, Input, Output, Receipt, Transaction},
    fuel_types::{AssetId, ContractId, Immediate18},
    fuel_vm::{
        consts::{REG_ONE, WORD_SIZE},
        prelude::Opcode,
        script_with_data_offset,
    },
};
use std::collections::HashMap;
use thiserror::Error;

use fuels_core::parameters::TxParameters;
use fuels_types::bech32::{Bech32Address, Bech32ContractId};
use fuels_types::errors::Error;

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
    ///   let (provider, _) = setup_test_provider(vec![], None).await;  
    ///   let tx = Transaction::default();
    ///   
    ///   let receipts = provider.send_transaction(&tx).await?;
    ///   dbg!(receipts);
    ///
    ///   Ok(())
    /// }
    /// ```
    pub async fn send_transaction(&self, tx: &Transaction) -> Result<Vec<Receipt>, Error> {
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
    ///     use std::net::SocketAddr;
    ///
    ///     // This is the address of a running node.
    ///     let server_address: SocketAddr = "127.0.0.1:4000"
    ///         .parse()
    ///         .expect("Unable to parse socket address");
    ///
    ///     // Create the provider using the client.
    ///     let provider = Provider::connect(server_address).await.unwrap();
    ///
    ///     // Create the wallet.
    ///     let _wallet = LocalWallet::new_random(Some(provider));
    /// }
    /// ```
    pub async fn connect(socket: SocketAddr) -> Result<Provider, Error> {
        Ok(Self {
            client: FuelClient::from(socket),
        })
    }

    pub async fn chain_info(&self) -> Result<ChainInfo, ProviderError> {
        Ok(self.client.chain_info().await?)
    }

    pub async fn dry_run(&self, tx: &Transaction) -> Result<Vec<Receipt>, ProviderError> {
        Ok(self.client.dry_run(tx).await?)
    }

    /// Gets all coins owned by address `from`, *even spent ones*. This returns actual coins
    /// (UTXOs).
    pub async fn get_coins(&self, from: &Bech32Address) -> Result<Vec<Coin>, ProviderError> {
        let mut coins: Vec<Coin> = vec![];

        let mut cursor = None;

        loop {
            let res = self
                .client
                .coins(
                    &from.hash().to_string(),
                    None,
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
        asset_id: AssetId,
        amount: u64,
    ) -> Result<Vec<Coin>, ProviderError> {
        let res = self
            .client
            .coins_to_spend(
                &from.hash().to_string(),
                vec![(format!("{:#x}", asset_id).as_str(), amount)],
                None,
                None,
            )
            .await?;
        Ok(res)
    }

    /// Craft a transaction used to transfer funds between two addresses.
    pub fn build_transfer_tx(
        &self,
        inputs: &[Input],
        outputs: &[Output],
        params: TxParameters,
    ) -> Transaction {
        // This script contains a single Opcode that returns immediately (RET)
        // since all this transaction does is move Inputs and Outputs around.
        let script = Opcode::RET(REG_ONE).to_bytes().to_vec();
        Transaction::Script {
            gas_price: params.gas_price,
            gas_limit: params.gas_limit,
            byte_price: params.byte_price,
            maturity: params.maturity,
            receipts_root: Default::default(),
            script,
            script_data: vec![],
            inputs: inputs.to_vec(),
            outputs: outputs.to_vec(),
            witnesses: vec![],
            metadata: None,
        }
    }

    /// Craft a transaction used to transfer funds to a contract.
    pub fn build_contract_transfer_tx(
        &self,
        to: ContractId,
        amount: u64,
        asset_id: AssetId,
        inputs: &[Input],
        outputs: &[Output],
        params: TxParameters,
    ) -> Transaction {
        let script_data: Vec<u8> = [
            to.to_vec(),
            amount.to_be_bytes().to_vec(),
            asset_id.to_vec(),
        ]
        .into_iter()
        .flatten()
        .collect();

        // This script loads:
        //  - a pointer to the contract id,
        //  - the actual amount
        //  - a pointer to the asset id
        // into the registers 0X10, 0x11, 0x12
        // and calls the TR instruction
        let (script, _) = script_with_data_offset!(
            data_offset,
            vec![
                Opcode::MOVI(0x10, data_offset as Immediate18),
                Opcode::MOVI(
                    0x11,
                    (data_offset as usize + ContractId::LEN) as Immediate18
                ),
                Opcode::LW(0x11, 0x11, 0),
                Opcode::MOVI(
                    0x12,
                    (data_offset as usize + ContractId::LEN + WORD_SIZE) as Immediate18
                ),
                Opcode::TR(0x10, 0x11, 0x12),
                Opcode::RET(REG_ONE)
            ],
            ConsensusParameters::DEFAULT.tx_offset()
        );
        #[allow(clippy::iter_cloned_collect)]
        let script = script.iter().copied().collect();

        Transaction::Script {
            gas_price: params.gas_price,
            gas_limit: params.gas_limit,
            byte_price: params.byte_price,
            maturity: params.maturity,
            receipts_root: Default::default(),
            script,
            script_data,
            inputs: inputs.to_vec(),
            outputs: outputs.to_vec(),
            witnesses: vec![],
            metadata: None,
        }
    }

    /// Get the balance of all spendable coins `asset_id` for address `address`. This is different
    /// from getting coins because we are just returning a number (the sum of UTXOs amount) instead
    /// of the UTXOs.
    pub async fn get_asset_balance(
        &self,
        address: &Bech32Address,
        asset_id: AssetId,
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
        asset_id: AssetId,
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
    ) -> Result<HashMap<String, u64>, ProviderError> {
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
                 }| (asset_id.to_string(), amount.try_into().unwrap()),
            )
            .collect();
        Ok(balances)
    }

    /// Get all balances of all assets for the contract with id `contract_id`.
    pub async fn get_contract_balances(
        &self,
        contract_id: &Bech32ContractId,
    ) -> Result<HashMap<String, u64>, ProviderError> {
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
                 }| (asset_id.to_string(), amount.try_into().unwrap()),
            )
            .collect();
        Ok(balances)
    }

    /// Get transaction by id.
    pub async fn get_transaction_by_id(
        &self,
        tx_id: &str,
    ) -> Result<TransactionResponse, ProviderError> {
        Ok(self.client.transaction(tx_id).await.unwrap().unwrap())
    }

    // - Get transaction(s)
    pub async fn get_transactions(
        &self,
        request: PaginationRequest<String>,
    ) -> Result<PaginatedResult<TransactionResponse, String>, ProviderError> {
        self.client.transactions(request).await.map_err(Into::into)
    }

    // Get transaction(s) by owner
    pub async fn get_transactions_by_owner(
        &self,
        owner: &Bech32Address,
        request: PaginationRequest<String>,
    ) -> Result<PaginatedResult<TransactionResponse, String>, ProviderError> {
        self.client
            .transactions_by_owner(&owner.hash().to_string(), request)
            .await
            .map_err(Into::into)
    }

    pub async fn latest_block_height(&self) -> Result<u64, ProviderError> {
        Ok(self.client.chain_info().await?.latest_block.height.0)
    }

    pub async fn produce_blocks(&self, amount: u64) -> io::Result<u64> {
        self.client.produce_block(amount).await
    }

    // @todo
    // - Get block(s)
}
