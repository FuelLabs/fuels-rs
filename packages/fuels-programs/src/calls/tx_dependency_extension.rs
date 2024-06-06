use fuels_core::types::{bech32::Bech32ContractId, errors::Result};

use crate::calls::utils::sealed;

/// How many times to attempt to resolve missing tx dependencies.
pub const DEFAULT_TX_DEP_ESTIMATION_ATTEMPTS: u64 = 10;

#[async_trait::async_trait]
pub trait TxDependencyExtension: Sized + sealed::Sealed {
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
    fn append_external_contract(self, contract_id: Bech32ContractId) -> Self;

    /// Simulates the call and attempts to resolve missing contract outputs.
    /// Forwards the received error if it cannot be fixed.
    async fn determine_missing_contracts(mut self, max_attempts: Option<u64>) -> Result<Self>;
}
