use crate::script::Script;
use anyhow::Result;
use fuel_gql_client::{
    fuel_tx::{Contract, Input, Output, Receipt, Transaction, UtxoId},
    fuel_types::{Address, AssetId},
    fuel_vm::{consts::REG_ONE, prelude::Opcode},
};
use fuels_core::errors::Error;
use fuels_signers::{wallet::Wallet, Signer};

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

    pub async fn create_predicate(
        &self,
        wallet: &Wallet,
        amount_to_predicate: u64,
        asset_id: AssetId,
    ) -> Result<Vec<Receipt>, Error> {
        let wallet_coins = wallet
            .get_asset_inputs_for_amount(
                asset_id,
                wallet.get_asset_balance(&asset_id).await.unwrap(),
                0,
            )
            .await?;

        let output_coin = Output::coin(self.address, amount_to_predicate, asset_id);
        let output_change = Output::change(wallet.address(), 0, asset_id);
        let mut tx = Transaction::script(
            1,
            1000000,
            1,
            0,
            Opcode::RET(REG_ONE).to_bytes().to_vec(),
            vec![],
            wallet_coins,
            vec![output_coin, output_change],
            vec![],
        );
        wallet.sign_transaction(&mut tx).await?;
        let provider = wallet.get_provider()?;
        Ok(provider.send_transaction(&tx).await?)
    }

    pub async fn spend_predicate(
        &self,
        wallet: &Wallet,
        amount_to_predicate: u64,
        asset_id: AssetId,
        receiver_address: Address,
        predicate_data: Option<Vec<u8>>,
    ) -> Result<(), Error> {
        let utxo_predicate_hash = wallet
            .get_provider()
            .unwrap()
            .get_spendable_coins(&self.address, asset_id, amount_to_predicate)
            .await?;

        let mut inputs = vec![];
        let mut total_amount_in_predicate = 0;

        for coin in utxo_predicate_hash {
            let input_coin = Input::coin_predicate(
                UtxoId::from(coin.utxo_id),
                coin.owner.into(),
                coin.amount.0,
                asset_id,
                0,
                self.code.clone(),
                match &predicate_data {
                    Some(data) => data.clone(),
                    None => vec![],
                },
            );
            inputs.push(input_coin);
            total_amount_in_predicate += coin.amount.0;
        }

        let output_coin = Output::coin(receiver_address, total_amount_in_predicate, asset_id);
        let output_change = Output::change(self.address, 0, asset_id);
        let new_tx = Transaction::script(
            0,
            1000000,
            0,
            0,
            vec![],
            vec![],
            inputs,
            vec![output_coin, output_change],
            vec![],
        );

        let script = Script::new(new_tx);
        let provider = wallet.get_provider()?;
        let _call_result = script.call(&provider.client).await;
        Ok(())
    }
}
