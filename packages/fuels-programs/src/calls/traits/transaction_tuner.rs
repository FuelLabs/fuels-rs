use fuels_accounts::Account;
use fuels_core::types::{
    errors::{error, Result},
    transaction::{ScriptTransaction, TxPolicies},
    transaction_builders::{
        BuildableTransaction, ScriptTransactionBuilder, TransactionBuilder, VariableOutputPolicy,
    },
};

use crate::calls::{
    utils::{build_tx_from_contract_calls, sealed, transaction_builder_from_contract_calls},
    ContractCall, ScriptCall,
};

#[async_trait::async_trait]
pub trait TransactionTuner: sealed::Sealed {
    async fn transaction_builder<T: Account>(
        &self,
        tx_policies: TxPolicies,
        variable_output_policy: VariableOutputPolicy,
        account: &T,
    ) -> Result<ScriptTransactionBuilder>;

    async fn build_tx<T: Account>(
        &self,
        tx_policies: TxPolicies,
        variable_output_policy: VariableOutputPolicy,
        account: &T,
    ) -> Result<ScriptTransaction>;
}

#[async_trait::async_trait]
impl TransactionTuner for ContractCall {
    async fn transaction_builder<T: Account>(
        &self,
        tx_policies: TxPolicies,
        variable_output_policy: VariableOutputPolicy,
        account: &T,
    ) -> Result<ScriptTransactionBuilder> {
        transaction_builder_from_contract_calls(
            std::slice::from_ref(self),
            tx_policies,
            variable_output_policy,
            account,
        )
        .await
    }

    async fn build_tx<T: Account>(
        &self,
        tx_policies: TxPolicies,
        variable_output_policy: VariableOutputPolicy,
        account: &T,
    ) -> Result<ScriptTransaction> {
        build_tx_from_contract_calls(
            std::slice::from_ref(self),
            tx_policies,
            variable_output_policy,
            account,
        )
        .await
    }
}

#[async_trait::async_trait]
impl TransactionTuner for ScriptCall {
    async fn transaction_builder<T: Account>(
        &self,
        tx_policies: TxPolicies,
        variable_output_policy: VariableOutputPolicy,
        _account: &T,
    ) -> Result<ScriptTransactionBuilder> {
        let (inputs, outputs) = self.prepare_inputs_outputs()?;

        Ok(ScriptTransactionBuilder::default()
            .with_variable_output_policy(variable_output_policy)
            .with_tx_policies(tx_policies)
            .with_script(self.script_binary.clone())
            .with_script_data(self.compute_script_data()?)
            .with_inputs(inputs)
            .with_outputs(outputs)
            .with_gas_estimation_tolerance(0.05))
    }

    async fn build_tx<T: Account>(
        &self,
        tx_policies: TxPolicies,
        variable_output_policy: VariableOutputPolicy,
        account: &T,
    ) -> Result<ScriptTransaction> {
        let mut tb = self
            .transaction_builder(tx_policies, variable_output_policy, account)
            .await?;

        account.add_witnesses(&mut tb)?;
        account.adjust_for_fee(&mut tb, 0).await?;

        tb.build(account.try_provider()?).await
    }
}

impl sealed::Sealed for Vec<ContractCall> {}

#[async_trait::async_trait]
impl TransactionTuner for Vec<ContractCall> {
    async fn transaction_builder<T: Account>(
        &self,
        tx_policies: TxPolicies,
        variable_output_policy: VariableOutputPolicy,
        account: &T,
    ) -> Result<ScriptTransactionBuilder> {
        validate_contract_calls(self)?;

        transaction_builder_from_contract_calls(self, tx_policies, variable_output_policy, account)
            .await
    }

    /// Returns the script that executes the contract calls
    async fn build_tx<T: Account>(
        &self,
        tx_policies: TxPolicies,
        variable_output_policy: VariableOutputPolicy,
        account: &T,
    ) -> Result<ScriptTransaction> {
        validate_contract_calls(self)?;

        build_tx_from_contract_calls(self, tx_policies, variable_output_policy, account).await
    }
}

fn validate_contract_calls(calls: &[ContractCall]) -> Result<()> {
    if calls.is_empty() {
        return Err(error!(
            Other,
            "no calls added. Have you used '.add_calls()'?"
        ));
    }

    Ok(())
}
