use fuel_tx::Receipt;
use fuels_core::types::{
    bech32::Bech32ContractId,
    errors::{transaction::Reason, Error, Result},
};

use crate::calls::utils::{find_id_of_missing_contract, is_missing_output_variables, sealed};

/// How many times to attempt to resolve missing tx dependencies.
pub const DEFAULT_TX_DEP_ESTIMATION_ATTEMPTS: u64 = 10;

#[async_trait::async_trait]
pub trait TxDependencyExtension: Sized + sealed::Sealed {
    async fn simulate(&mut self) -> Result<()>;

    /// Appends `num` [`fuel_tx::Output::Variable`]s to the transaction.
    /// Note that this is a builder method, i.e. use it as a chain:
    ///
    /// ```ignore
    /// my_contract_instance.my_method(...).append_variable_outputs(num).call()
    /// my_script_instance.main(...).append_variable_outputs(num).call()
    /// ```
    ///
    /// [`Output::Variable`]: fuel_tx::Output::Variable
    fn append_variable_outputs(self, num: u64) -> Self;

    /// Appends additional external contracts as dependencies to this call.
    /// Effectively, this will be used to create additional
    /// [`fuel_tx::Input::Contract`]/[`fuel_tx::Output::Contract`]
    /// pairs and set them into the transaction. Note that this is a builder
    /// method, i.e. use it as a chain:
    ///
    /// ```ignore
    /// my_contract_instance.my_method(...).append_contract(additional_contract_id).call()
    /// my_script_instance.main(...).append_contract(additional_contract_id).call()
    /// ```
    ///
    /// [`Input::Contract`]: fuel_tx::Input::Contract
    /// [`Output::Contract`]: fuel_tx::Output::Contract
    fn append_contract(self, contract_id: Bech32ContractId) -> Self;

    fn append_missing_dependencies(mut self, receipts: &[Receipt]) -> Self {
        if is_missing_output_variables(receipts) {
            self = self.append_variable_outputs(1);
        }
        if let Some(contract_id) = find_id_of_missing_contract(receipts) {
            self = self.append_contract(contract_id);
        }

        self
    }

    /// Simulates the call and attempts to resolve missing tx dependencies.
    /// Forwards the received error if it cannot be fixed.
    async fn estimate_tx_dependencies(mut self, max_attempts: Option<u64>) -> Result<Self> {
        let attempts = max_attempts.unwrap_or(DEFAULT_TX_DEP_ESTIMATION_ATTEMPTS);

        for _ in 0..attempts {
            match self.simulate().await {
                Ok(_) => return Ok(self),

                Err(Error::Transaction(Reason::Reverted { ref receipts, .. })) => {
                    self = self.append_missing_dependencies(receipts);
                }

                Err(other_error) => return Err(other_error),
            }
        }

        self.simulate().await.map(|_| self)
    }
}
