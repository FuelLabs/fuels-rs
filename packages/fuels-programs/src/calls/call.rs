use fuel_abi_types::error_codes::FAILED_TRANSFER_TO_ADDRESS_SIGNAL;
use fuel_tx::{Output, Receipt};
use fuel_types::{Address, AssetId};
use fuel_vm::fuel_asm::PanicReason;

use fuels_types::bech32::Bech32ContractId;

use crate::logs::LogDecoder;
use crate::{calls::contract_call::ContractCall, calls::script_call::ScriptCall};

// Trait implemented by contract instances so that
// they can be passed to the `set_contracts` method
pub trait SettableContract {
    fn id(&self) -> Bech32ContractId;
    fn log_decoder(&self) -> LogDecoder;
}

macro_rules! implement_shared_call_methods {
    ($target:ty) => {
        impl $target {
            pub fn with_external_contracts(self, external_contracts: Vec<Bech32ContractId>) ->
            Self {
                Self {
                    external_contracts,
                    ..self
                }
            }

            pub fn append_variable_outputs(&mut self, num: u64) {
                let new_variable_outputs = vec![
                    Output::Variable {
                        amount: 0,
                        to: Address::zeroed(),
                        asset_id: AssetId::default(),
                    };
                    num as usize
                ];
                self.outputs.extend(new_variable_outputs)
            }

            pub fn append_external_contracts(&mut self, contract_id: Bech32ContractId) {
                self.external_contracts.push(contract_id)
            }

            pub fn append_message_outputs(&mut self, num: u64) {
                let new_message_outputs = vec![
                    Output::Message {
                        recipient: Address::zeroed(),
                        amount: 0,
                    };
                    num as usize
                ];
                self.outputs.extend(new_message_outputs)
            }

            pub fn is_missing_output_variables(receipts: &[Receipt]) -> bool {
                receipts.iter().any(
                    |r| matches!(r, Receipt::Revert { ra, .. } if *ra == FAILED_TRANSFER_TO_ADDRESS_SIGNAL),
                )
            }

            pub fn find_contract_not_in_inputs(receipts: &[Receipt]) -> Option<&Receipt> {
                receipts.iter().find(
                    |r| matches!(r, Receipt::Panic { reason, .. } if *reason.reason() == PanicReason::ContractNotInInputs ),
                )
            }
        }

    };
}

implement_shared_call_methods!(ScriptCall);
implement_shared_call_methods!(ContractCall);
