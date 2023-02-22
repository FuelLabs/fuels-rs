use std::collections::HashMap;
use std::fmt::Debug;

use fuel_tx::field::Salt;
use fuel_tx::InputRepr::Contract;
use fuel_tx::{Input, Output, Receipt, TxPointer, UtxoId};
use fuel_types::{AssetId, Bytes32, ContractId};

use fuels_types::transaction::Transaction;

use fuels_core::{abi_encoder::UnresolvedBytes, offsets};
use fuels_signers::wallet::WalletError;
use fuels_signers::{provider::Provider, Account, PayFee};
use fuels_types::bech32::{Bech32Address, Bech32ContractId};
use fuels_types::coin::Coin;
use fuels_types::constants::BASE_ASSET_ID;
use fuels_types::errors::Error;
use fuels_types::errors::Result;
use fuels_types::message::Message;
use fuels_types::parameters::TxParameters;
use fuels_types::resource::Resource;
use fuels_types::transaction::ScriptTransaction;

#[derive(Debug, Clone)]
pub struct Predicate {
    pub address: Bech32Address,
    pub code: Vec<u8>,
    pub data: UnresolvedBytes,
    pub provider: Option<Provider>,
}

type PredicateResult<T> = std::result::Result<T, WalletError>;

impl Predicate {
    pub fn provider(&self) -> PredicateResult<&Provider> {
        self.provider.as_ref().ok_or(WalletError::NoProvider)
    }

    pub async fn get_balances(&self) -> Result<HashMap<String, u64>> {
        self.provider()?
            .get_balances(&self.address)
            .await
            .map_err(Into::into)
    }

    pub fn set_provider(&mut self, provider: Provider) {
        self.provider = Some(provider)
    }

    pub fn address(&self) -> &Bech32Address {
        &self.address
    }

    pub async fn get_asset_inputs_for_amount_predicates<T: Transaction>(
        &self,
        tx: &mut T,
        asset_id: AssetId,
        amount: u64,
    ) -> Result<Vec<Input>> {
        let consensus_parameters = self.provider()?.chain_info().await?.consensus_parameters;

        dbg!(offsets::contract_input_offset());

        let mut offset = tx.base_offset(&consensus_parameters);
        dbg!(&offset);
            // offsets::base_offset(&consensus_parameters)
            //     consensus_parameters.tx_offset() + fuel_tx::Create::salt_offset_static() + Bytes32::LEN;

        // let mut offset =
        // offsets::base_offset(&consensus_parameters)
        //     offsets::contract_input_offset();

        let inputs = self
            .get_spendable_resources(asset_id, amount)
            .await?
            .into_iter()
            .map(|resource| match resource {
                Resource::Coin(coin) => {
                    offset += offsets::coin_predicate_data_offset(self.code.len());
                    let data = self.data.clone().resolve(offset as u64);
                    offset += data.len();
                    self.create_coin_predicate(coin, asset_id, self.code.clone(), data)
                }
                Resource::Message(message) => {
                    offset +=
                        offsets::message_predicate_data_offset(message.data.len(), self.code.len());
                    let data = self.data.clone().resolve(offset as u64);
                    offset += data.len();
                    self.create_message_predicate(message, self.code.clone(), data)
                }
            })
            .collect::<Vec<Input>>();
        Ok(inputs)
    }

    /// Returns a vector containing the output coin and change output given an asset and amount
    pub fn get_asset_outputs_for_amount(
        &self,
        to: &Bech32Address,
        asset_id: AssetId,
        amount: u64,
    ) -> Vec<Output> {
        vec![
            Output::coin(to.into(), amount, asset_id),
            // Note that the change will be computed by the node.
            // Here we only have to tell the node who will own the change and its asset ID.
            Output::change((&self.address).into(), 0, asset_id),
        ]
    }

    pub async fn get_asset_inputs_for_amount(
        &self,
        asset_id: AssetId,
        amount: u64,
        witness_index: u8,
    ) -> Result<Vec<Input>> {
        Ok(self
            .get_spendable_resources(asset_id, amount)
            .await?
            .into_iter()
            .map(|resource| match resource {
                Resource::Coin(coin) => self.create_coin_input(coin, asset_id, witness_index),
                Resource::Message(message) => self.create_message_input(message, witness_index),
            })
            .collect::<Vec<Input>>())
    }

    fn create_coin_input(&self, coin: Coin, asset_id: AssetId, witness_index: u8) -> Input {
        Input::coin_signed(
            coin.utxo_id,
            coin.owner.into(),
            coin.amount,
            asset_id,
            TxPointer::default(),
            witness_index,
            0,
        )
    }

    fn create_message_input(&self, message: Message, witness_index: u8) -> Input {
        Input::message_signed(
            message.message_id(),
            message.sender.into(),
            message.recipient.into(),
            message.amount,
            message.nonce,
            witness_index,
            message.data,
        )
    }

    fn create_coin_predicate(
        &self,
        coin: Coin,
        asset_id: AssetId,
        code: Vec<u8>,
        predicate_data: Vec<u8>,
    ) -> Input {
        Input::coin_predicate(
            coin.utxo_id,
            coin.owner.into(),
            coin.amount,
            asset_id,
            TxPointer::new(0, 0),
            0,
            code,
            predicate_data,
        )
    }

    fn create_message_predicate(
        &self,
        message: Message,
        code: Vec<u8>,
        predicate_data: Vec<u8>,
    ) -> Input {
        Input::message_predicate(
            message.message_id(),
            message.sender.into(),
            message.recipient.into(),
            message.amount,
            message.nonce,
            message.data,
            code,
            predicate_data,
        )
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl PayFee for Predicate {
    async fn pay_fee_resources<Tx: fuels_types::transaction::Transaction + Send + std::fmt::Debug>(
        &self,
        tx: &mut Tx,
        previous_base_amount: u64,
        _witness_index: u8,
    ) -> PredicateResult<()> {

        let consensus_parameters = self.provider()?.chain_info().await?.consensus_parameters;
        let transaction_fee = tx
            .fee_checked_from_tx(&consensus_parameters)
            .expect("Error calculating TransactionFee");
        let (base_asset_inputs, remaining_inputs): (Vec<_>, Vec<_>) = tx.inputs().iter().cloned().partition(|input| {
            matches!(input , Input::MessageSigned { .. }) || matches!(input , Input::CoinSigned { asset_id , .. } if asset_id == &BASE_ASSET_ID) });

        let base_inputs_sum: u64 = base_asset_inputs
            .iter()
            .map(|input| input.amount().unwrap())
            .sum();
        if base_inputs_sum < previous_base_amount {
            return Err(fuels_signers::wallet::WalletError::LowAmount(
                Error::WalletError(
                    "The provided base asset amount is less than the present input coins"
                        .to_string(),
                ),
            ));
        }

        let mut new_base_amount = transaction_fee.total() + previous_base_amount;
        let is_consuming_utxos = tx
            .inputs()
            .iter()
            .any(|input| !matches!(input, Input::Contract { .. }));
        const MIN_AMOUNT: u64 = 1;
        if !is_consuming_utxos && new_base_amount == 0 {
            new_base_amount = MIN_AMOUNT;
        }
        //TODO find out is it Contract deploy or sscript

        let new_base_inputs = self
            .get_asset_inputs_for_amount_predicates(tx, BASE_ASSET_ID, new_base_amount)
            .await?;

        let adjusted_inputs: ::std::vec::Vec<_> = remaining_inputs
            .into_iter()
            .chain(new_base_inputs.into_iter())
            .collect();

        *tx.inputs_mut() = adjusted_inputs;
        let is_base_change_present = tx.outputs().iter().any(|output| {
            matches!(output , Output::Change { asset_id , .. }
                                        if asset_id == & BASE_ASSET_ID)
        });

        if !is_base_change_present && new_base_amount != 0 {
            tx.outputs_mut()
                .push(Output::change(self.address().into(), 0, BASE_ASSET_ID));
        }

        dbg!(&tx);


        Ok(())
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Account for Predicate {
    type Error = WalletError;

    fn address(&self) -> &Bech32Address {
        &self.address
    }

    fn get_provider(&self) -> std::result::Result<&Provider, Self::Error> {
        self.provider.as_ref().ok_or(WalletError::NoProvider)
    }

    fn set_provider(&mut self, provider: Provider) {
        self.set_provider(provider)
    }

    async fn get_spendable_resources(
        &self,
        asset_id: AssetId,
        amount: u64,
    ) -> std::result::Result<Vec<Resource>, Self::Error> {
        self.provider()?
            .get_spendable_resources(&self.address, asset_id, amount)
            .await
            .map_err(Into::into)
    }

    async fn transfer(
        &self,
        to: &Bech32Address,
        amount: u64,
        asset_id: AssetId,
        tx_parameters: Option<TxParameters>,
    ) -> std::result::Result<(String, Vec<Receipt>), Self::Error> {
        // let inputs = self
        //     .get_asset_inputs_for_amount_predicates(asset_id, amount)
        //     .await?;

        let inputs = self
            .get_asset_inputs_for_amount(asset_id, amount, 0)
            .await?;

        let outputs = self.get_asset_outputs_for_amount(to, asset_id, amount);

        let mut tx = ScriptTransaction::new(inputs, outputs, tx_parameters.unwrap_or_default());

        // if we are not transferring the base asset, previous base amount is 0
        if asset_id == AssetId::default() {
            self.pay_fee_resources(&mut tx, amount, 0).await?;
        } else {
            self.pay_fee_resources(&mut tx, 0, 0).await?;
        };

        let receipts = self.get_provider()?.send_transaction(&tx).await?;

        Ok((tx.id().to_string(), receipts))
    }

    async fn force_transfer_to_contract(
        &self,
        to: &Bech32ContractId,
        balance: u64,
        asset_id: AssetId,
        tx_parameters: TxParameters,
    ) -> std::result::Result<(String, Vec<Receipt>), Self::Error> {
        let zeroes = Bytes32::zeroed();
        let plain_contract_id: ContractId = to.into();

        let mut inputs = vec![Input::contract(
            UtxoId::new(zeroes, 0),
            zeroes,
            zeroes,
            TxPointer::default(),
            plain_contract_id,
        )];
        // Todo fix this
        // inputs.extend(
        //     self.get_asset_inputs_for_amount_predicates(, asset_id, balance)
        //         .await?,
        // );

        let outputs = vec![
            Output::contract(0, zeroes, zeroes),
            Output::change((&self.address).into(), 0, asset_id),
        ];

        // Build transaction and sign it
        let mut tx = ScriptTransaction::build_contract_transfer_tx(
            plain_contract_id,
            balance,
            asset_id,
            inputs,
            outputs,
            tx_parameters,
        );

        // if we are not transferring the base asset, previous base amount is 0
        let base_amount = if asset_id == AssetId::default() {
            balance
        } else {
            0
        };

        self.pay_fee_resources(&mut tx, base_amount, 0).await?;

        let tx_id = tx.id();
        let receipts = self.get_provider()?.send_transaction(&tx).await?;

        Ok((tx_id.to_string(), receipts))
    }

    fn convert_to_signed_resources(&self, spendable_resources: Vec<Resource>) -> Vec<Input> {
        let mut offset = 0;

        spendable_resources
            .into_iter()
            .map(|resource| match resource {
                Resource::Coin(coin) => {
                    offset += offsets::coin_predicate_data_offset(self.code.len());

                    let data = self.data.clone().resolve(offset as u64);
                    offset += data.len();

                    self.create_coin_predicate(coin.clone(), coin.asset_id, self.code.clone(), data)
                }
                Resource::Message(message) => {
                    offset +=
                        offsets::message_predicate_data_offset(message.data.len(), self.code.len());

                    let data = self.data.clone().resolve(offset as u64);
                    offset += data.len();

                    self.create_message_predicate(message, self.code.clone(), data)
                }
            })
            .collect::<Vec<_>>()
    }
}
