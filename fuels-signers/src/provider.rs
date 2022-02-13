use std::io;

use fuel_gql_client::client::schema::coin::Coin;
use fuel_gql_client::client::{schema::HexString256, FuelClient, PageDirection, PaginationRequest};
use fuel_tx::{Address, Bytes32, Bytes64, Input, Output, Transaction, UtxoId};

use fuel_vm::prelude::Opcode;
use thiserror::Error;

/// An error involving a signature.
#[derive(Debug, Error)]
pub enum ProviderError {
    #[error("Request failed: {0}")]
    TransactionRequestError(String),
    #[error(transparent)]
    ClientRequestError(#[from] io::Error),
}

/// Encapsulates common client operations in the SDK.
/// Note that you may also use `client`, which is an instance
/// of `FuelClient`, directly, which providers a broader API.
pub struct Provider {
    pub client: FuelClient,
}

impl Provider {
    pub fn new(client: FuelClient) -> Self {
        Self { client }
    }

    /// Shallow wrapper on client's submit
    pub async fn send_transaction(&self, tx: &Transaction) -> io::Result<HexString256> {
        self.client.submit(tx).await
    }

    pub async fn get_coins(&self, from: &Address) -> Result<Vec<Coin>, ProviderError> {
        let res = self
            .client
            .coins(
                &from.to_string(),
                None,
                PaginationRequest {
                    cursor: None,
                    results: 100,
                    direction: PageDirection::Forward,
                },
            )
            .await?;

        Ok(res.results)
    }

    /// Transfer funds between `from` and `to`.
    pub async fn transfer(
        &self,
        from: &Address,
        to: &Address,
        amount: u64,
        utxo: UtxoId,
    ) -> io::Result<Bytes32> {
        let script = Opcode::RET(0x10).to_bytes().to_vec();
        let tx = Transaction::Script {
            gas_price: 0,
            gas_limit: 1_000_000,
            byte_price: 0,
            maturity: 0,
            receipts_root: Default::default(),
            script,
            script_data: vec![],
            inputs: vec![Input::Coin {
                utxo_id: utxo, // <--- temp
                owner: *from,
                amount,
                color: Default::default(),
                witness_index: 0,
                maturity: 0,
                predicate: vec![],
                predicate_data: vec![],
            }],
            outputs: vec![Output::Coin {
                amount,
                to: *to,
                color: Default::default(),
            }],
            witnesses: vec![vec![].into()],
            metadata: None,
        };
        self.send_transaction(&tx).await.map(Into::into)
    }

    // @todo
    // - Get transaction(s)
    // - Get block(s)
}
