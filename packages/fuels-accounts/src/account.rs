use std::collections::HashMap;

use async_trait::async_trait;
use fuel_core_client::client::pagination::{PaginatedResult, PaginationRequest};
use fuel_tx::{Output, TxId, TxPointer, UtxoId};
use fuels_core::types::{
    Address, AssetId, Bytes32, ContractId, Nonce,
    coin::Coin,
    coin_type::CoinType,
    coin_type_id::CoinTypeId,
    errors::{Context, Result},
    input::Input,
    message::Message,
    transaction::{Transaction, TxPolicies},
    transaction_builders::{BuildableTransaction, ScriptTransactionBuilder, TransactionBuilder},
    transaction_response::TransactionResponse,
    tx_response::TxResponse,
    tx_status::Success,
};

use crate::{
    accounts_utils::{
        add_base_change_if_needed, available_base_assets_and_amount, calculate_missing_base_amount,
        extract_message_nonce, split_into_utxo_ids_and_nonces,
    },
    provider::{Provider, ResourceFilter},
};

#[derive(Clone, Debug)]
pub struct WithdrawToBaseResponse {
    pub tx_status: Success,
    pub tx_id: TxId,
    pub nonce: Nonce,
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait ViewOnlyAccount: Send + Sync {
    fn address(&self) -> Address;

    fn try_provider(&self) -> Result<&Provider>;

    async fn get_transactions(
        &self,
        request: PaginationRequest<String>,
    ) -> Result<PaginatedResult<TransactionResponse, String>> {
        Ok(self
            .try_provider()?
            .get_transactions_by_owner(&self.address(), request)
            .await?)
    }

    /// Gets all unspent coins of asset `asset_id` owned by the account.
    async fn get_coins(&self, asset_id: AssetId) -> Result<Vec<Coin>> {
        Ok(self
            .try_provider()?
            .get_coins(&self.address(), asset_id)
            .await?)
    }

    /// Get the balance of all spendable coins `asset_id` for address `address`. This is different
    /// from getting coins because we are just returning a number (the sum of UTXOs amount) instead
    /// of the UTXOs.
    async fn get_asset_balance(&self, asset_id: &AssetId) -> Result<u128> {
        self.try_provider()?
            .get_asset_balance(&self.address(), asset_id)
            .await
    }

    /// Gets all unspent messages owned by the account.
    async fn get_messages(&self) -> Result<Vec<Message>> {
        Ok(self.try_provider()?.get_messages(&self.address()).await?)
    }

    /// Get all the spendable balances of all assets for the account. This is different from getting
    /// the coins because we are only returning the sum of UTXOs coins amount and not the UTXOs
    /// coins themselves.
    async fn get_balances(&self) -> Result<HashMap<String, u128>> {
        self.try_provider()?.get_balances(&self.address()).await
    }

    /// Get some spendable resources (coins and messages) of asset `asset_id` owned by the account
    /// that add up at least to amount `amount`. The returned coins (UTXOs) are actual coins that
    /// can be spent. The number of UXTOs is optimized to prevent dust accumulation.
    async fn get_spendable_resources(
        &self,
        asset_id: AssetId,
        amount: u128,
        excluded_coins: Option<Vec<CoinTypeId>>,
    ) -> Result<Vec<CoinType>> {
        let (excluded_utxos, excluded_message_nonces) =
            split_into_utxo_ids_and_nonces(excluded_coins);

        let filter = ResourceFilter {
            from: self.address(),
            asset_id: Some(asset_id),
            amount,
            excluded_utxos,
            excluded_message_nonces,
        };

        self.try_provider()?.get_spendable_resources(filter).await
    }

    /// Returns a vector containing the output coin and change output given an asset and amount
    fn get_asset_outputs_for_amount(
        &self,
        to: Address,
        asset_id: AssetId,
        amount: u64,
    ) -> Vec<Output> {
        vec![
            Output::coin(to, amount, asset_id),
            // Note that the change will be computed by the node.
            // Here we only have to tell the node who will own the change and its asset ID.
            Output::change(self.address(), 0, asset_id),
        ]
    }

    /// Returns a vector consisting of `Input::Coin`s and `Input::Message`s for the given
    /// asset ID and amount.
    async fn get_asset_inputs_for_amount(
        &self,
        asset_id: AssetId,
        amount: u128,
        excluded_coins: Option<Vec<CoinTypeId>>,
    ) -> Result<Vec<Input>>;

    /// Add base asset inputs to the transaction to cover the estimated fee
    /// and add a change output for the base asset if needed.
    /// Requires contract inputs to be at the start of the transactions inputs vec
    /// so that their indexes are retained
    async fn adjust_for_fee<Tb: TransactionBuilder + Sync>(
        &self,
        tb: &mut Tb,
        used_base_amount: u128,
    ) -> Result<()> {
        let provider = self.try_provider()?;
        let consensus_parameters = provider.consensus_parameters().await?;
        let base_asset_id = consensus_parameters.base_asset_id();
        let (base_assets, base_amount) = available_base_assets_and_amount(tb, base_asset_id);
        let missing_base_amount =
            calculate_missing_base_amount(tb, base_amount, used_base_amount, provider).await?;

        if missing_base_amount > 0 {
            let new_base_inputs = self
                .get_asset_inputs_for_amount(
                    *consensus_parameters.base_asset_id(),
                    missing_base_amount,
                    Some(base_assets),
                )
                .await
                .with_context(|| {
                    format!("failed to get base asset ({base_asset_id}) inputs with amount: `{missing_base_amount}`")
                })?;

            tb.inputs_mut().extend(new_base_inputs);
        };

        add_base_change_if_needed(tb, self.address(), *consensus_parameters.base_asset_id());

        Ok(())
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait Account: ViewOnlyAccount {
    // Add signatures to the builder if the underlying account is a wallet
    fn add_witnesses<Tb: TransactionBuilder>(&self, _tb: &mut Tb) -> Result<()> {
        Ok(())
    }

    /// Transfer funds from this account to another `Address`.
    /// Fails if amount for asset ID is larger than address's spendable coins.
    /// Returns the transaction ID that was sent and the list of receipts.
    async fn transfer(
        &self,
        to: Address,
        amount: u64,
        asset_id: AssetId,
        tx_policies: TxPolicies,
    ) -> Result<TxResponse> {
        let provider = self.try_provider()?;

        let inputs = self
            .get_asset_inputs_for_amount(asset_id, amount.into(), None)
            .await?;
        let outputs = self.get_asset_outputs_for_amount(to, asset_id, amount);

        let mut tx_builder =
            ScriptTransactionBuilder::prepare_transfer(inputs, outputs, tx_policies);

        self.add_witnesses(&mut tx_builder)?;

        let consensus_parameters = provider.consensus_parameters().await?;
        let used_base_amount = if asset_id == *consensus_parameters.base_asset_id() {
            amount.into()
        } else {
            0
        };
        self.adjust_for_fee(&mut tx_builder, used_base_amount)
            .await
            .context("failed to adjust inputs to cover for missing base asset")?;

        let tx = tx_builder.build(provider).await?;
        let tx_id = tx.id(consensus_parameters.chain_id());

        let tx_status = provider.send_transaction_and_await_commit(tx).await?;

        Ok(TxResponse {
            tx_status: tx_status.take_success_checked(None)?,
            tx_id,
        })
    }

    /// Unconditionally transfers `balance` of type `asset_id` to
    /// the contract at `to`.
    /// Fails if balance for `asset_id` is larger than this account's spendable balance.
    /// Returns the corresponding transaction ID and the list of receipts.
    ///
    /// CAUTION !!!
    ///
    /// This will transfer coins to a contract, possibly leading
    /// to the PERMANENT LOSS OF COINS if not used with care.
    async fn force_transfer_to_contract(
        &self,
        to: ContractId,
        balance: u64,
        asset_id: AssetId,
        tx_policies: TxPolicies,
    ) -> Result<TxResponse> {
        let provider = self.try_provider()?;

        let zeroes = Bytes32::zeroed();

        let mut inputs = vec![Input::contract(
            UtxoId::new(zeroes, 0),
            zeroes,
            zeroes,
            TxPointer::default(),
            to,
        )];

        inputs.extend(
            self.get_asset_inputs_for_amount(asset_id, balance.into(), None)
                .await?,
        );

        let outputs = vec![
            Output::contract(0, zeroes, zeroes),
            Output::change(self.address(), 0, asset_id),
        ];

        // Build transaction and sign it
        let mut tb = ScriptTransactionBuilder::prepare_contract_transfer(
            to,
            balance,
            asset_id,
            inputs,
            outputs,
            tx_policies,
        );

        let consensus_parameters = provider.consensus_parameters().await?;
        let used_base_amount = if asset_id == *consensus_parameters.base_asset_id() {
            balance
        } else {
            0
        };

        self.add_witnesses(&mut tb)?;
        self.adjust_for_fee(&mut tb, used_base_amount.into())
            .await
            .context("failed to adjust inputs to cover for missing base asset")?;

        let tx = tb.build(provider).await?;

        let consensus_parameters = provider.consensus_parameters().await?;
        let tx_id = tx.id(consensus_parameters.chain_id());

        let tx_status = provider.send_transaction_and_await_commit(tx).await?;

        Ok(TxResponse {
            tx_status: tx_status.take_success_checked(None)?,
            tx_id,
        })
    }

    /// Withdraws an amount of the base asset to
    /// an address on the base chain.
    /// Returns the transaction ID, message ID and the list of receipts.
    async fn withdraw_to_base_layer(
        &self,
        to: Address,
        amount: u64,
        tx_policies: TxPolicies,
    ) -> Result<WithdrawToBaseResponse> {
        let provider = self.try_provider()?;
        let consensus_parameters = provider.consensus_parameters().await?;

        let inputs = self
            .get_asset_inputs_for_amount(*consensus_parameters.base_asset_id(), amount.into(), None)
            .await?;

        let mut tb = ScriptTransactionBuilder::prepare_message_to_output(
            to,
            amount,
            inputs,
            tx_policies,
            *consensus_parameters.base_asset_id(),
        );

        self.add_witnesses(&mut tb)?;
        self.adjust_for_fee(&mut tb, amount.into())
            .await
            .context("failed to adjust inputs to cover for missing base asset")?;

        let tx = tb.build(provider).await?;
        let tx_id = tx.id(consensus_parameters.chain_id());

        let tx_status = provider.send_transaction_and_await_commit(tx).await?;
        let success = tx_status.take_success_checked(None)?;

        let nonce = extract_message_nonce(&success.receipts)
            .expect("MessageId could not be retrieved from tx receipts.");

        Ok(WithdrawToBaseResponse {
            tx_status: success,
            tx_id,
            nonce,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use fuel_crypto::{Message, SecretKey, Signature};
    use fuel_tx::{Address, ConsensusParameters, Output, Transaction as FuelTransaction};
    use fuels_core::{
        traits::Signer,
        types::{DryRun, DryRunner, transaction::Transaction},
    };

    use super::*;
    use crate::signers::private_key::PrivateKeySigner;

    #[derive(Default)]
    struct MockDryRunner {
        c_param: ConsensusParameters,
    }

    #[cfg_attr(not(target_arch = "wasm32"), async_trait)]
    impl DryRunner for MockDryRunner {
        async fn dry_run(&self, _: FuelTransaction) -> Result<DryRun> {
            Ok(DryRun {
                succeeded: true,
                script_gas: 0,
                variable_outputs: 0,
            })
        }

        async fn consensus_parameters(&self) -> Result<ConsensusParameters> {
            Ok(self.c_param.clone())
        }

        async fn estimate_gas_price(&self, _block_header: u32) -> Result<u64> {
            Ok(0)
        }

        async fn estimate_predicates(
            &self,
            _: &FuelTransaction,
            _: Option<u32>,
        ) -> Result<FuelTransaction> {
            unimplemented!()
        }
    }

    #[tokio::test]
    async fn sign_tx_and_verify() -> std::result::Result<(), Box<dyn std::error::Error>> {
        // ANCHOR: sign_tb
        let secret = SecretKey::from_str(
            "5f70feeff1f229e4a95e1056e8b4d80d0b24b565674860cc213bdb07127ce1b1",
        )?;
        let signer = PrivateKeySigner::new(secret);

        // Set up a transaction
        let mut tb = {
            let input_coin = Input::ResourceSigned {
                resource: CoinType::Coin(Coin {
                    amount: 10000000,
                    owner: signer.address(),
                    ..Default::default()
                }),
            };

            let output_coin = Output::coin(
                Address::from_str(
                    "0xc7862855b418ba8f58878db434b21053a61a2025209889cc115989e8040ff077",
                )?,
                1,
                Default::default(),
            );
            let change = Output::change(signer.address(), 0, Default::default());

            ScriptTransactionBuilder::prepare_transfer(
                vec![input_coin],
                vec![output_coin, change],
                Default::default(),
            )
        };

        // Add `Signer` to the transaction builder
        tb.add_signer(signer.clone())?;
        // ANCHOR_END: sign_tb

        let tx = tb.build(MockDryRunner::default()).await?; // Resolve signatures and add corresponding witness indexes

        // Extract the signature from the tx witnesses
        let bytes = <[u8; Signature::LEN]>::try_from(tx.witnesses().first().unwrap().as_ref())?;
        let tx_signature = Signature::from_bytes(bytes);

        // Sign the transaction manually
        let message = Message::from_bytes(*tx.id(0.into()));
        let signature = signer.sign(message).await?;

        // Check if the signatures are the same
        assert_eq!(signature, tx_signature);

        // Check if the signature is what we expect it to be
        assert_eq!(
            signature,
            Signature::from_str(
                "faa616776a1c336ef6257f7cb0cb5cd932180e2d15faba5f17481dae1cbcaf314d94617bd900216a6680bccb1ea62438e4ca93b0d5733d33788ef9d79cc24e9f"
            )?
        );

        // Recover the address that signed the transaction
        let recovered_address = signature.recover(&message)?;

        assert_eq!(*signer.address(), *recovered_address.hash());

        // Verify signature
        signature.verify(&recovered_address, &message)?;

        Ok(())
    }
}
