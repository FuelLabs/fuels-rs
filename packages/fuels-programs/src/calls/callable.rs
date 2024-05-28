use fuel_tx::Receipt;
use fuels_accounts::Account;
use fuels_core::{
    codec::DecoderConfig,
    types::{
        bech32::Bech32ContractId,
        errors::Result,
        param_types::ParamType,
        transaction::{ScriptTransaction, TxPolicies},
        transaction_builders::{
            BuildableTransaction, ScriptTransactionBuilder, TransactionBuilder,
        },
        Token,
    },
};

use crate::calls::{
    receipt_parser::ReceiptParser,
    utils::{
        build_tx_from_contract_calls, new_variable_outputs, sealed,
        transaction_builder_from_contract_calls,
    },
    ContractCall, ScriptCall,
};

#[async_trait::async_trait]
pub trait Callable: sealed::Sealed {
    fn with_external_contracts(self, external_contracts: Vec<Bech32ContractId>) -> Self;

    async fn transaction_builder<T: Account>(
        &self,
        tx_policies: TxPolicies,
        account: &T,
    ) -> Result<ScriptTransactionBuilder>;

    async fn build_tx<T: Account>(
        &self,
        tx_policies: TxPolicies,
        account: &T,
    ) -> Result<ScriptTransaction>;

    fn parse_token(
        &self,
        receipts: &[Receipt],
        decoder_config: DecoderConfig,
        param_type: &ParamType,
    ) -> Result<Token>;

    fn append_variable_outputs(&mut self, num: u64);

    fn append_contract(&mut self, contract_id: Bech32ContractId);
}

impl sealed::Sealed for ContractCall {}

#[async_trait::async_trait]
impl Callable for ContractCall {
    fn with_external_contracts(self, external_contracts: Vec<Bech32ContractId>) -> Self {
        ContractCall {
            external_contracts,
            ..self
        }
    }

    async fn transaction_builder<T: Account>(
        &self,
        tx_policies: TxPolicies,
        account: &T,
    ) -> Result<ScriptTransactionBuilder> {
        transaction_builder_from_contract_calls(std::slice::from_ref(self), tx_policies, account)
            .await
    }

    async fn build_tx<T: Account>(
        &self,
        tx_policies: TxPolicies,
        account: &T,
    ) -> Result<ScriptTransaction> {
        build_tx_from_contract_calls(std::slice::from_ref(self), tx_policies, account).await
    }

    fn parse_token(
        &self,
        receipts: &[Receipt],
        decoder_config: DecoderConfig,
        param_type: &ParamType,
    ) -> Result<Token> {
        ReceiptParser::new(receipts, decoder_config).parse_call(&self.contract_id, param_type)
    }

    fn append_variable_outputs(&mut self, num: u64) {
        self.variable_outputs
            .extend(new_variable_outputs(num as usize));
    }

    fn append_contract(&mut self, contract_id: Bech32ContractId) {
        self.external_contracts.push(contract_id)
    }
}

impl sealed::Sealed for ScriptCall {}

#[async_trait::async_trait]
impl Callable for ScriptCall {
    fn with_external_contracts(self, external_contracts: Vec<Bech32ContractId>) -> Self {
        ScriptCall {
            external_contracts,
            ..self
        }
    }

    async fn transaction_builder<T: Account>(
        &self,
        tx_policies: TxPolicies,
        _account: &T,
    ) -> Result<ScriptTransactionBuilder> {
        let (inputs, outputs) = self.prepare_inputs_outputs()?;

        Ok(ScriptTransactionBuilder::default()
            .with_tx_policies(tx_policies)
            .with_script(self.script_binary.clone())
            .with_script_data(self.compute_script_data()?)
            .with_inputs(inputs)
            .with_outputs(outputs))
    }

    async fn build_tx<T: Account>(
        &self,
        tx_policies: TxPolicies,
        account: &T,
    ) -> Result<ScriptTransaction> {
        let mut tb = self.transaction_builder(tx_policies, account).await?;

        account.add_witnesses(&mut tb)?;
        account.adjust_for_fee(&mut tb, 0).await?;

        tb.build(account.try_provider()?).await
    }

    fn parse_token(
        &self,
        receipts: &[Receipt],
        decoder_config: DecoderConfig,
        param_type: &ParamType,
    ) -> Result<Token> {
        ReceiptParser::new(receipts, decoder_config).parse_script(param_type)
    }

    fn append_variable_outputs(&mut self, num: u64) {
        self.variable_outputs
            .extend(new_variable_outputs(num as usize));
    }

    fn append_contract(&mut self, contract_id: Bech32ContractId) {
        self.external_contracts.push(contract_id)
    }
}
