use fuels_accounts::Account;
use fuels_core::types::{
    errors::{Result, error},
    transaction::{ScriptTransaction, TxPolicies},
    transaction_builders::{
        BuildableTransaction, ScriptTransactionBuilder, TransactionBuilder, VariableOutputPolicy,
    },
};

use crate::{
    DEFAULT_MAX_FEE_ESTIMATION_TOLERANCE,
    calls::{
        ContractCall, ScriptCall,
        utils::{build_with_tb, sealed, transaction_builder_from_contract_calls},
    },
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
        tb: ScriptTransactionBuilder,
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
        tb: ScriptTransactionBuilder,
        account: &T,
    ) -> Result<ScriptTransaction> {
        build_with_tb(std::slice::from_ref(self), tb, account).await
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
            .with_gas_estimation_tolerance(DEFAULT_MAX_FEE_ESTIMATION_TOLERANCE)
            .with_max_fee_estimation_tolerance(DEFAULT_MAX_FEE_ESTIMATION_TOLERANCE))
    }

    async fn build_tx<T: Account>(
        &self,
        mut tb: ScriptTransactionBuilder,
        account: &T,
    ) -> Result<ScriptTransaction> {
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
        tb: ScriptTransactionBuilder,
        account: &T,
    ) -> Result<ScriptTransaction> {
        validate_contract_calls(self)?;

        build_with_tb(self, tb, account).await
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
