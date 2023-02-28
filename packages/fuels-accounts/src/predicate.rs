use std::fmt::Debug;

use fuel_tx::{Input, Output, Receipt, TxPointer, UtxoId};
use fuel_types::{AssetId, Bytes32, ContractId};

use fuels_types::transaction::Transaction;

use crate::accounts_utils::{
    create_coin_input, create_coin_predicate, create_message_input, create_message_predicate,
    extract_message_id,
};
use crate::provider::Provider;
use crate::{Account, AccountError, AccountResult};
use fuels_core::offsets::base_offset;
use fuels_core::{abi_encoder::UnresolvedBytes, offsets};
use fuels_types::bech32::{Bech32Address, Bech32ContractId};
use fuels_types::constants::BASE_ASSET_ID;
use fuels_types::errors::Error;
use fuels_types::errors::Result;
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

type PredicateResult<T> = std::result::Result<T, AccountError>;

impl Predicate {
    pub fn provider(&self) -> PredicateResult<&Provider> {
        self.provider.as_ref().ok_or(AccountError::NoProvider)
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
        let mut offset = tx.tx_offset();

        let inputs = self
            .get_spendable_resources(asset_id, amount)
            .await?
            .into_iter()
            .map(|resource| match resource {
                Resource::Coin(coin) => {
                    offset += offsets::coin_predicate_data_offset(self.code.len());
                    let data = self.data.clone().resolve(offset as u64);
                    offset += data.len();
                    create_coin_predicate(coin, asset_id, self.code.clone(), data)
                }
                Resource::Message(message) => {
                    offset +=
                        offsets::message_predicate_data_offset(message.data.len(), self.code.len());
                    let data = self.data.clone().resolve(offset as u64);
                    offset += data.len();
                    create_message_predicate(message, self.code.clone(), data)
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
                Resource::Coin(coin) => create_coin_input(coin, asset_id, witness_index),
                Resource::Message(message) => create_message_input(message, witness_index),
            })
            .collect::<Vec<Input>>())
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Account for Predicate {
    fn address(&self) -> &Bech32Address {
        &self.address
    }

    fn get_provider(&self) -> AccountResult<&Provider> {
        self.provider.as_ref().ok_or(AccountError::NoProvider)
    }

    fn set_provider(&mut self, provider: Provider) {
        self.set_provider(provider)
    }

    async fn pay_fee_resources<Tx: Transaction + Send + Debug>(
        &self,
        tx: &mut Tx,
        previous_base_amount: u64,
        _witness_index: u8,
    ) -> std::result::Result<(), Error> {
        let consensus_parameters = self.provider()?.chain_info().await?.consensus_parameters;
        let transaction_fee = tx
            .fee_checked_from_tx(&consensus_parameters)
            .expect("Error calculating TransactionFee");

        let (base_asset_inputs, remaining_inputs): (Vec<_>, Vec<_>) = tx.inputs().iter().cloned().partition(|input| {
            matches!(input , Input::CoinPredicate { asset_id , .. } if asset_id == &BASE_ASSET_ID) ||
            matches!(input , Input::MessagePredicate { .. })
        });

        let base_inputs_sum: u64 = base_asset_inputs
            .iter()
            .map(|input| input.amount().unwrap())
            .sum();

        if base_inputs_sum < previous_base_amount {
            return Err(AccountError::LowAmount(Error::AccountError(
                "The provided base asset amount is less than the present input coins".to_string(),
            ))
            .into());
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

        Ok(())
    }

    async fn transfer(
        &self,
        to: &Bech32Address,
        amount: u64,
        asset_id: AssetId,
        tx_parameters: Option<TxParameters>,
    ) -> std::result::Result<(String, Vec<Receipt>), Error> {
        let inputs = self
            .get_asset_inputs_for_amount(asset_id, amount, 0)
            .await?;

        let outputs = self.get_asset_outputs_for_amount(to, asset_id, amount);

        let mut tx = ScriptTransaction::new(inputs, outputs, tx_parameters.unwrap_or_default());

        let consensus_parameters = self.provider()?.chain_info().await?.consensus_parameters;
        tx.tx_offset = offsets::base_offset(&consensus_parameters);

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
    ) -> std::result::Result<(String, Vec<Receipt>), Error> {
        let zeroes = Bytes32::zeroed();
        let plain_contract_id: ContractId = to.into();

        let mut inputs = vec![Input::contract(
            UtxoId::new(zeroes, 0),
            zeroes,
            zeroes,
            TxPointer::default(),
            plain_contract_id,
        )];

        inputs.extend(
            self.get_asset_inputs_for_amount(asset_id, balance, 0)
                .await?,
        );

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

        let consensus_parameters = self
            .provider
            .as_ref()
            .expect("No provider available")
            .consensus_parameters()
            .await?;

        let script_offset = base_offset(&consensus_parameters);
        tx.tx_offset = script_offset
            + tx.script_data().len()
            + tx.script().len()
            + offsets::contract_input_offset();

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

    async fn withdraw_to_base_layer(
        &self,
        to: &Bech32Address,
        amount: u64,
        tx_parameters: TxParameters,
    ) -> std::result::Result<(String, String, Vec<Receipt>), Error> {
        let inputs = self
            .get_asset_inputs_for_amount(BASE_ASSET_ID, amount, 0)
            .await?;

        let mut tx =
            ScriptTransaction::build_message_to_output_tx(to.into(), amount, inputs, tx_parameters);

        let consensus_parameters = self
            .get_provider()?
            .chain_info()
            .await?
            .consensus_parameters;

        let script_offset = base_offset(&consensus_parameters);
        tx.tx_offset = script_offset + tx.script_data().len() + tx.script().len() - 64;

        self.pay_fee_resources(&mut tx, amount, 0).await?;

        let tx_id = tx.id().to_string();
        let receipts = self.get_provider()?.send_transaction(&tx).await?;

        let message_id = extract_message_id(&receipts)
            .expect("MessageId could not be retrieved from tx receipts.");

        Ok((tx_id, message_id.to_string(), receipts))
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

                    create_coin_predicate(coin.clone(), coin.asset_id, self.code.clone(), data)
                }
                Resource::Message(message) => {
                    offset +=
                        offsets::message_predicate_data_offset(message.data.len(), self.code.len());

                    let data = self.data.clone().resolve(offset as u64);
                    offset += data.len();

                    create_message_predicate(message, self.code.clone(), data)
                }
            })
            .collect::<Vec<_>>()
    }
}
