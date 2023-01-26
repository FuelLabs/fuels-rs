use fuel_tx::{Output, Receipt};
use fuel_types::{Address, AssetId};
use fuel_vm::fuel_asm::PanicReason;

use fuels_core::{constants::FAILED_TRANSFER_TO_ADDRESS_SIGNAL, parameters::CallParameters};
use fuels_types::bech32::Bech32ContractId;

use crate::logs::LogDecoder;
use crate::{calls::contract_call::ContractCall, calls::script_call::ScriptCall};

// Trait implemented by contract instances so that
// they can be passed to the `set_contracts` method
pub trait SettableContract {
    fn id(&self) -> Bech32ContractId;
    fn log_decoder(&self) -> LogDecoder;
}

pub(crate) trait ProgramCall {
    fn with_external_contracts(self, external_contracts: &[Bech32ContractId]) -> Self;
    fn with_call_parameters(self, call_parameters: CallParameters) -> Self;
    fn with_variable_outputs(self, variable_outputs: &[Output]) -> Self;
    fn with_message_outputs(self, message_outputs: &[Output]) -> Self;
    fn append_variable_outputs(&mut self, num: u64);
    fn append_external_contracts(&mut self, contract_id: Bech32ContractId);
    fn append_message_outputs(&mut self, num: u64);
    fn is_missing_output_variables(receipts: &[Receipt]) -> bool {
        receipts.iter().any(
            |r| matches!(r, Receipt::Revert { ra, .. } if *ra == FAILED_TRANSFER_TO_ADDRESS_SIGNAL),
        )
    }
    fn find_contract_not_in_inputs(receipts: &[Receipt]) -> Option<&Receipt> {
        receipts.iter().find(
            |r| matches!(r, Receipt::Panic { reason, .. } if *reason.reason() == PanicReason::ContractNotInInputs ),
        )
    }
}

macro_rules! impl_programcall_trait_methods {
    ($target:ty) => {
        impl ProgramCall for $target {
            fn with_external_contracts(self, external_contracts: &[Bech32ContractId]) -> Self {
                Self {
                    external_contracts: external_contracts.to_vec(),
                    ..self
                }
            }

            fn with_call_parameters(self, call_parameters: CallParameters) -> Self {
                Self {
                    call_parameters,
                    ..self
                }
            }

            fn with_variable_outputs(self, variable_outputs: &[Output]) -> Self {
                Self {
                    variable_outputs: variable_outputs.to_vec(),
                    ..self
                }
            }

            fn with_message_outputs(self, message_outputs: &[Output]) -> Self {
                Self {
                    message_outputs: message_outputs.to_vec(),
                    ..self
                }
            }

            fn append_variable_outputs(&mut self, num: u64) {
                let new_variable_outputs = vec![
                    Output::Variable {
                        amount: 0,
                        to: Address::zeroed(),
                        asset_id: AssetId::default(),
                    };
                    num as usize
                ];
                self.variable_outputs.extend(new_variable_outputs)
            }

            fn append_external_contracts(&mut self, contract_id: Bech32ContractId) {
                self.external_contracts.push(contract_id)
            }

            fn append_message_outputs(&mut self, num: u64) {
                let new_message_outputs = vec![
                    Output::Message {
                        recipient: Address::zeroed(),
                        amount: 0,
                    };
                    num as usize
                ];
                self.message_outputs.extend(new_message_outputs)
            }
        }
    };
}

impl_programcall_trait_methods!(ContractCall);
impl_programcall_trait_methods!(ScriptCall);
