use std::fs::read;
use std::{collections::HashSet, fmt::Debug, marker::PhantomData};

use fuel_tx::field::{Inputs, Outputs, Witnesses};
use fuel_tx::{
    field, Cacheable, Chargeable, Contract, ContractId, Input, Output, Receipt, Transaction,
    TransactionFee, TxPointer, UniqueIdentifier,
};
use fuel_types::bytes::padded_len_usize;
use fuel_types::{Address, AssetId};
use itertools::chain;

use fuels_core::abi_encoder::ABIEncoder;
use fuels_core::constants::BASE_ASSET_ID;
use fuels_core::{
    abi_encoder::UnresolvedBytes,
    offsets,
    offsets::base_offset,
    parameters::{CallParameters, TxParameters},
};
use fuels_signers::wallet::WalletError;
use fuels_signers::{provider::Provider, Account, PayFee, Signer, WalletUnlocked};
use fuels_types::bech32::Bech32Address;
use fuels_types::coin::Coin;
use fuels_types::errors::Error;
use fuels_types::message::Message;
use fuels_types::resource::Resource;
use fuels_types::{
    bech32::Bech32ContractId,
    errors::Result,
    traits::{Parameterize, Tokenizable},
    B512,
};

use crate::{
    call_response::FuelCallResponse,
    call_utils::{generate_contract_inputs, generate_contract_outputs},
    contract::{get_decoded_output, SettableContract},
    execution_script::ExecutableFuelCall,
    logs::{map_revert_error, LogDecoder},
};

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

    pub fn set_provider(&mut self, provider: Provider) {
        self.provider = Some(provider)
    }

    pub fn address(&self) -> &Bech32Address {
        &self.address
    }

    pub async fn get_asset_inputs_for_amount_predicates(
        &self,
        asset_id: AssetId,
        amount: u64,
    ) -> Result<Vec<Input>> {
        let consensus_parameters = self.provider()?.chain_info().await?.consensus_parameters;

        let mut offset = offsets::base_offset(&consensus_parameters);

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

        dbg!(&inputs);

        Ok(inputs)
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
    type Error = WalletError;

    fn address(&self) -> &Bech32Address {
        &self.address
    }

    async fn pay_fee_resources<
        'a_t,
        Tx: Chargeable + Inputs + Outputs + Send + Cacheable + UniqueIdentifier + Witnesses,
    >(
        &'a_t self,
        tx: &'a_t mut Tx,
        previous_base_amount: u64,
        witness_index: u8,
    ) -> PredicateResult<()> {
        dbg!(&self.data);

        let consensus_parameters = self.provider()?.chain_info().await?.consensus_parameters;
        let transaction_fee = TransactionFee::checked_from_tx(&consensus_parameters, tx)
            .expect("Error calculating TransactionFee");
        let (base_asset_inputs, remaining_inputs): (Vec<_>, Vec<_>) = tx.inputs().iter().cloned().partition(|input| {
            matches!(input , Input::MessageSigned { .. }) || matches!(input , Input::CoinSigned { asset_id , .. } if asset_id == &BASE_ASSET_ID) });

        let base_inputs_sum: u64 = base_asset_inputs
            .iter()
            .map(|input| input.amount().unwrap())
            .sum();
        if base_inputs_sum < previous_base_amount {
            return Err(fuels_signers::wallet::WalletError::LowAmount(
                Error::WalletError(format!(
                    "The provided base asset amount is less than the present input coins"
                )),
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

        let new_base_inputs = self
            .get_asset_inputs_for_amount_predicates(BASE_ASSET_ID, new_base_amount)
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

    fn get_provider(&self) -> PredicateResult<&Provider> {
        self.provider.as_ref().ok_or(WalletError::NoProvider)
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Account for Predicate {
    fn address(&self) -> &Bech32Address {
        &self.address
    }

    fn get_provider(&self) -> std::result::Result<&Provider, <Self as PayFee>::Error> {
        self.provider.as_ref().ok_or(WalletError::NoProvider)
    }

    fn set_provider(&mut self, provider: Provider) {
        self.set_provider(provider)
    }

    async fn get_spendable_resources(
        &self,
        asset_id: AssetId,
        amount: u64,
    ) -> std::result::Result<Vec<Resource>, <Self as PayFee>::Error> {
        self.provider()?
            .get_spendable_resources(&self.address, asset_id, amount)
            .await
            .map_err(Into::into)
    }
}
