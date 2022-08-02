use crate::script::Script;
use anyhow::Result;
use fuel_gql_client::{
    fuel_tx::{Contract, Input, Output, Receipt, Transaction, UtxoId},
    fuel_types::{Address, AssetId},
};

use fuel_gql_client::fuel_vm::prelude::Opcode;
use fuels_core::parameters::TxParameters;
use fuels_signers::{provider::Provider, wallet::Wallet, Signer};
use fuels_types::{bech32::Bech32Address, errors::Error};

/// Predicate provides methods to create new predicates and call them
pub struct Predicate {
    pub address: Address,
    code: Vec<u8>,
}

impl Predicate {
    pub fn new(code: Vec<u8>) -> Self {
        let address = (*Contract::root_from_code(&code)).into();
        Self { code, address }
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
        let script = Opcode::RET(0x10).to_bytes().to_vec();
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

    /// Deploys locked coins in a Predicate to the given wallet's provider.
    ///
    /// # Arguments
    ///
    /// * `wallet` - A wallet that funds this transaction
    /// * `coin_amount_to_predicate` - The amount of locked coins as given asset id to store within Predicate
    /// * `asset_id` - The asset id of the locked coins stored within Predicate
    pub async fn deploy_predicate(
        &self,
        wallet: &Wallet,
        coin_amount_to_predicate: u64,
        asset_id: AssetId,
    ) -> Result<Vec<Receipt>, Error> {
        let wallet_coins = wallet
            .get_asset_inputs_for_amount(
                asset_id,
                wallet.get_asset_balance(&asset_id).await.unwrap(),
                0,
            )
            .await?;

        let output_coin = Output::coin(self.address, coin_amount_to_predicate, asset_id);
        let output_change = Output::change(wallet.address().into(), 0, asset_id);

        let mut tx = self.build_transfer_tx(
            &wallet_coins,
            &[output_coin, output_change],
            TxParameters::default(),
        );
        wallet.sign_transaction(&mut tx).await?;
        let provider = wallet.get_provider()?;
        provider.send_transaction(&tx).await
    }

    /// Attempts to spend coins from referenced Predicate and add to the given wallet's coins
    ///
    /// # Arguments
    ///
    /// * `provider` - A provider to handle the transaction
    /// * `coin_amount_to_predicate` - The amount of locked coins as given asset id to retrieve within Predicate
    /// * `asset_id` - The asset id of the locked coins stored within Predicate
    /// * `receiver_address` - The address that may receive the locked coins if Predicate returns true
    /// * `predicate_data` - Optional parameter data to be sent to Predicate function as part of processing
    pub async fn spend_predicate(
        &self,
        provider: &Provider,
        coin_amount_to_predicate: u64,
        asset_id: AssetId,
        receiver_address: &Bech32Address,
        predicate_data: Option<Vec<u8>>,
    ) -> Result<Vec<Receipt>, Error> {
        let spendable_predicate_coins = provider
            .get_spendable_coins(&self.address.into(), asset_id, coin_amount_to_predicate)
            .await?;

        let mut inputs = vec![];
        let mut total_amount_in_predicate = 0;

        let predicate_data = predicate_data.unwrap_or_default();
        for coin in spendable_predicate_coins {
            let input_coin = Input::coin_predicate(
                UtxoId::from(coin.utxo_id),
                coin.owner.into(),
                coin.amount.0,
                asset_id,
                0,
                self.code.clone(),
                predicate_data.clone(),
            );
            inputs.push(input_coin);
            total_amount_in_predicate += coin.amount.0;
        }

        let output_coin =
            Output::coin(receiver_address.into(), total_amount_in_predicate, asset_id);
        let output_change = Output::change(self.address, 0, asset_id);

        let tx = self.build_transfer_tx(
            &inputs,
            &[output_coin, output_change],
            TxParameters::default(),
        );

        let script = Script::new(tx);
        script.call(provider).await
    }
}
