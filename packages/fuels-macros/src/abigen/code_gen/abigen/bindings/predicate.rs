use std::collections::HashSet;

use proc_macro2::{Ident, TokenStream};
use quote::quote;

use crate::{
    abigen::code_gen::{
        abi_types::{FullProgramABI, FullTypeDeclaration},
        abigen::bindings::{function_generator::FunctionGenerator, utils::extract_main_fn},
        generated_code::GeneratedCode,
        type_path::TypePath,
    },
    error::Result,
};

pub(crate) fn predicate_bindings(
    name: &Ident,
    abi: FullProgramABI,
    no_std: bool,
    shared_types: &HashSet<FullTypeDeclaration>,
) -> Result<GeneratedCode> {
    if no_std {
        return Ok(GeneratedCode::default());
    }

    let encode_function = expand_fn(&abi, shared_types)?;

    let code = quote! {

        use ::std::boxed::Box;

        #[cfg_attr(not(target_arch = "wasm32"), ::async_trait::async_trait)]
        impl ::fuels::signers::Account for #name {
            type Error = ::fuels::types::errors::Error;

            fn address(&self) -> &::fuels::types::bech32::Bech32Address {
                &self.address
            }

            fn get_provider(&self) -> ::fuels::types::errors::Result<&::fuels::signers::provider::Provider> {
                self.provider()
            }

            fn set_provider(&mut self, provider: ::fuels::signers::provider::Provider) {
                self.set_provider(::std::option::Option::Some(provider))
            }
        }


        #[cfg_attr(not(target_arch = "wasm32"), ::async_trait::async_trait)]
        impl ::fuels::signers::PayFee for #name {
            type Error = ::fuels::types::errors::Error;

            fn address(&self) -> &::fuels::prelude::Bech32Address {
                &self.address
            }

            async fn pay_fee_resources<
                'a_t,
                Tx: ::fuels::tx::Chargeable
                    + ::fuels::tx::field::Inputs
                    + ::fuels::tx::field::Outputs
                    + ::std::marker::Send
                    + ::fuels::tx::Cacheable
                    + ::fuels::tx::UniqueIdentifier
                    + ::fuels::tx::field::Witnesses,
            >(
                &'a_t self,
                tx: &'a_t mut Tx,
                previous_base_amount: u64,
                // witness_index: u8, in predicat witnes is 0
            ) -> ::fuels::types::errors::Result<()> {

                // ::std::boxed::Box::pin(async move {

                    let consensus_parameters = self
                        .get_provider()?
                        .chain_info()
                        .await?
                        .consensus_parameters;
                    let transaction_fee = ::fuels::tx::TransactionFee::checked_from_tx(&consensus_parameters, tx)
                        .expect("Error calculating TransactionFee");

                    let (base_asset_inputs, remaining_inputs): (::std::vec::Vec<_>, ::std::vec::Vec<_>) =
                        tx.inputs().iter().cloned().partition(|input| {
                            ::std::matches!(input, ::fuels::tx::Input::MessageSigned { .. })
                                || ::std::matches!(input, ::fuels::tx::Input::CoinSigned { asset_id, .. } if asset_id == &::fuels::core::constants::BASE_ASSET_ID)
                        });

                    let base_inputs_sum: u64 = base_asset_inputs
                        .iter()
                        .map(|input| input.amount().unwrap())
                        .sum();

                    if base_inputs_sum < previous_base_amount {
                         return ::std::result::Result::Err(
                             ::fuels::types::errors::Error::WalletError(::std::format!("The provided base asset amount is less than the present input coins")))
                    }

                    let mut new_base_amount = transaction_fee.total() + previous_base_amount;
                    // If the tx doesn't consume any UTXOs, attempting to repeat it will lead to an
                    // error due to non unique tx ids (e.g. repeated contract call with configured gas cost of 0).
                    // Here we enforce a minimum amount on the base asset to avoid this
                    let is_consuming_utxos = tx
                        .inputs()
                        .iter()
                        .any(|input| !::std::matches!(input, ::fuels::tx::Input::Contract { .. }));
                    const MIN_AMOUNT: u64 = 1;
                    if !is_consuming_utxos && new_base_amount == 0 {
                        new_base_amount = MIN_AMOUNT;
                    }

                    let new_base_inputs = self
                    .get_asset_inputs_for_amount(::fuels::core::constants::BASE_ASSET_ID, new_base_amount, 0) // i set this to 0
                    .await?;
                    let adjusted_inputs: ::std::vec::Vec<_> = remaining_inputs
                        .into_iter()
                        .chain(new_base_inputs.into_iter())
                        .collect();
                    *tx.inputs_mut() = adjusted_inputs;

                    let is_base_change_present = tx.outputs().iter().any(|output| {
                        ::std::matches!(output, ::fuels::tx::Output::Change { asset_id, .. } if asset_id == &::fuels::core::constants::BASE_ASSET_ID)
                    });
                    // add a change output for the base asset if it doesn't exist and there are base inputs
                    if !is_base_change_present && new_base_amount != 0 {
                        tx.outputs_mut()
                            .push(::fuels::tx::Output::change(self.address().into(), 0, ::fuels::core::constants::BASE_ASSET_ID));
                    }

                    ::std::result::Result::Ok(())

               // }).await
            }

            fn get_provider(&self) -> ::fuels::types::errors::Result<&::fuels::signers::provider::Provider> {
                self.provider()
            }

        }

        #[derive(Debug)]
        pub struct #name {
            address: ::fuels::types::bech32::Bech32Address,
            code: ::std::vec::Vec<u8>,
            data: ::fuels::core::abi_encoder::UnresolvedBytes,
            provider: ::std::option::Option<::fuels::prelude::Provider>
        }

        impl #name {
            pub fn new(code: ::std::vec::Vec<u8>) -> Self {
                let address: ::fuels::types::Address = (*::fuels::tx::Contract::root_from_code(&code)).into();
                Self {
                    address: address.clone().into(),
                    code,
                    data: ::fuels::core::abi_encoder::UnresolvedBytes::new(),
                    provider: ::std::option::Option::None
                }
            }

            pub fn load_from(file_path: &str) -> ::fuels::types::errors::Result<Self> {
                ::std::result::Result::Ok(Self::new(::std::fs::read(file_path)?))
            }

            pub fn address(&self) -> &::fuels::types::bech32::Bech32Address {
                &self.address
            }

            pub fn code(&self) -> ::std::vec::Vec<u8> {
                self.code.clone()
            }

            pub fn provider(&self) -> ::fuels::types::errors::Result<&::fuels::signers::provider::Provider> {
                self.provider.as_ref().ok_or(::fuels::types::errors::Error::from(
                    ::fuels::signers::wallet::WalletError::NoProvider
                ))
            }

            pub fn set_provider(&mut self, provider: ::std::option::Option<::fuels::prelude::Provider>) {
                self.provider = provider
            }

            pub fn data(&self) -> ::fuels::core::abi_encoder::UnresolvedBytes {
                self.data.clone()
            }

            pub async fn receive(&self, from: &::fuels::signers::wallet::WalletUnlocked,
                                 amount: u64,
                                 asset_id: ::fuels::types::AssetId,
                                 tx_parameters: ::std::option::Option<::fuels::core::parameters::TxParameters>
            ) -> ::fuels::types::errors::Result<(::std::string::String, ::std::vec::Vec<::fuels::tx::Receipt>)> {
                let tx_parameters = tx_parameters.unwrap_or_default();
                from
                    .transfer(
                        self.address(),
                        amount,
                        asset_id,
                        tx_parameters
                    )
                    .await
            }

            pub async fn spend(&self, to: &::fuels::signers::wallet::WalletUnlocked,
                                amount: u64,
                                asset_id: ::fuels::types::AssetId,
                                tx_parameters: ::std::option::Option<::fuels::core::parameters::TxParameters>
            ) -> ::fuels::types::errors::Result<::std::vec::Vec<::fuels::tx::Receipt>> {
                let tx_parameters = tx_parameters.unwrap_or_default();
                to
                    .receive_from_predicate(
                        self.address(),
                        self.code(),
                        amount,
                        asset_id,
                        self.data(),
                        tx_parameters,
                    )
                    .await
            }

            pub async fn get_asset_inputs_for_amount(
                &self,
                asset_id: ::fuels::types::AssetId,
                amount: u64,
                witness_index: u8,
            ) -> ::fuels::types::errors::Result<::std::vec::Vec<::fuels::tx::Input>> {
                ::std::result::Result::Ok(self
                    .get_spendable_resources(asset_id, amount)
                    .await?
                    .into_iter()
                    .map(|resource|
                        match resource {
                        ::fuels::types::resource::Resource::Coin(coin) => self.create_coin_input(coin, asset_id, witness_index),
                        ::fuels::types::resource::Resource::Message(message) => self.create_message_input(message, witness_index),
                    })
                    .collect::<::std::vec::Vec<::fuels::tx::Input>>())
            }

            pub async fn get_spendable_resources(
                &self,
                asset_id: ::fuels::types::AssetId,
                amount: u64,
            ) -> ::fuels::types::errors::Result<::std::vec::Vec<::fuels::types::resource::Resource>> {
                self.provider()?
                    .get_spendable_resources(&self.address, asset_id, amount)
                    .await
                    .map_err(::std::convert::Into::into)
            }

            fn create_coin_input(&self, coin: ::fuels::types::coin::Coin, asset_id: ::fuels::types::AssetId, witness_index: u8) ->
            ::fuels::tx::Input {
                ::fuels::tx::Input::coin_signed(
                    coin.utxo_id,
                    coin.owner.into(),
                    coin.amount,
                    asset_id,
                    ::fuels::tx::TxPointer::new(0,0),
                    witness_index,
                    0,
                )
            }

            fn create_message_input(&self, message: ::fuels::types::message::Message, witness_index: u8) -> ::fuels::tx::Input {
                ::fuels::tx::Input::message_signed(
                    message.message_id(),
                    message.sender.into(),
                    message.recipient.into(),
                    message.amount,
                    message.nonce,
                    witness_index,
                    message.data,
                )
            }

           #encode_function
        }
    };
    // All publicly available types generated above should be listed here.
    let type_paths = [TypePath::new(name).expect("We know name is not empty.")].into();

    Ok(GeneratedCode {
        code,
        usable_types: type_paths,
    })
}

fn expand_fn(
    abi: &FullProgramABI,
    shared_types: &HashSet<FullTypeDeclaration>,
) -> Result<TokenStream> {
    let fun = extract_main_fn(&abi.functions)?;
    let mut generator = FunctionGenerator::new(fun, shared_types)?;

    let arg_tokens = generator.tokenized_args();

    let body = quote! {
        let data = ::fuels::core::abi_encoder::ABIEncoder::encode(&#arg_tokens).expect("Cannot encode predicate data");

        Self {
            address: self.address.clone(),
            code: self.code.clone(),
            data,
            provider: self.provider.clone()
        }
    };

    generator
        .set_doc("Run the predicate's encode function with the provided arguments".to_string())
        .set_name("encode_data".to_string())
        .set_output_type(quote! {Self})
        .set_body(body);

    Ok(generator.into())
}
