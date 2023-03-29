use fuel_tx::{ContractId, Receipt};
use fuels_core::abi_decoder::ABIDecoder;
use fuels_types::{
    bech32::Bech32ContractId,
    error,
    errors::{Error, Result},
    param_types::{ParamType, ReturnLocation},
    Token,
};
use itertools::Itertools;

pub mod call_response;
pub mod call_utils;
pub mod contract;
pub mod logs;
pub mod script_calls;

#[derive(Debug, Clone, Default)]
pub struct Configurables {
    offsets_with_data: Vec<(u64, Vec<u8>)>,
}

impl Configurables {
    pub fn new(offsets_with_data: Vec<(u64, Vec<u8>)>) -> Self {
        Self { offsets_with_data }
    }

    pub fn update_constants_in(&self, binary: &mut [u8]) {
        for (offset, data) in &self.offsets_with_data {
            let offset = *offset as usize;
            binary[offset..offset + data.len()].copy_from_slice(data)
        }
    }
}

pub struct ReceiptDecoder {
    receipts: Vec<Receipt>,
}

impl ReceiptDecoder {
    pub fn new(receipts: &[Receipt]) -> Self {
        let relevant_receipts: Vec<Receipt> = receipts
            .iter()
            .filter(|receipt| {
                matches!(receipt, Receipt::ReturnData { .. } | Receipt::Return { .. })
            })
            .cloned()
            .collect();

        Self {
            receipts: relevant_receipts,
        }
    }

    /// Based on receipts returned by a script transaction, the contract ID (in the case of a contract call),
    /// and the output param, decode the values and return them.
    pub fn try_decode_output(
        &mut self,
        contract_id: Option<&Bech32ContractId>,
        output_param: &ParamType,
    ) -> Result<Token> {
        let null_contract_id = ContractId::new([0u8; 32]);
        let contract_id: ContractId = match contract_id {
            Some(contract_id) => contract_id.into(),
            // During a script execution, the script's contract id is the **null** contract id
            None => null_contract_id,
        };

        let encoded_value = match output_param.get_return_location() {
            ReturnLocation::ReturnData if output_param.is_vm_heap_type() => {
                self.extract_return_data_heap(contract_id)
            }
            ReturnLocation::ReturnData => self.extract_return_data(contract_id),
            ReturnLocation::Return => self.extract_return(contract_id),
        };

        if let Some(value) = encoded_value {
            //self.receipts.remove(index);
            let decoded_value = ABIDecoder::decode_single(output_param, &value)?;
            Ok(decoded_value)
        } else {
            return Err(error!(InvalidData, "ReceiptDecoder: failed to find matching receipts entry for {:?}", output_param));
        }
    }

    fn extract_return_data(&mut self, contract_id: ContractId) -> Option<Vec<u8>> {
        let (index, data) = self.receipts
            .iter()
            .enumerate()
            .find_map(|(i, receipt)| {
                if matches!(receipt,
                    Receipt::ReturnData { id, .. } if *id == contract_id) {
                    Some((i, receipt.data().expect("ReturnData should have data").to_vec()))
                } else {
                    None
                }
        })?;

        self.receipts.remove(index);
        Some(data)
    }

    fn extract_return(&mut self, contract_id: ContractId) -> Option<Vec<u8>> {
        let (index, data) = self.receipts
            .iter()
            .enumerate()
            .find_map(|(i, receipt)| {
                if matches!(receipt,
                    Receipt::Return { id, ..} if *id == contract_id) {
                    Some((i, receipt.val().expect("Return should have val").to_be_bytes().to_vec()))
                } else {
                    None
                }
            })?;
    
        self.receipts.remove(index);
        Some(data)
    }

    fn extract_return_data_heap(&mut self, contract_id: ContractId) -> Option<Vec<u8>> {
        // If the output of the function is a vector, then there are 2 consecutive ReturnData
        // receipts. The first one is the one that returns the pointer to the vec struct in the
        // VM memory, the second one contains the actual vector bytes (that the previous receipt
        // points to).
        // We ensure to take the right "first" ReturnData receipt by checking for the
        // contract_id. There are no receipts in between the two ReturnData receipts because of
        // the way the scripts are built (the calling script adds a RETD just after the CALL
        // opcode, see `get_single_call_instructions`).
        if let Some ((i, vector_data)) = self
            .receipts
            .iter()
            .enumerate()
            .tuple_windows()
            .find_map(|((i, current_receipt), (_, next_receipt))| {
                let vec_data = Self::extract_vec_data(current_receipt, next_receipt, contract_id)?;
                Some((i, vec_data.clone()))
            })
        {
            self.receipts.remove(i);
            // the index has adjusted due to the previous removal
            self.receipts.remove(i);
            Some(vector_data)
        }
        else {
            None
        } 
    }

    fn extract_vec_data<'a>(
        current_receipt: &Receipt,
        next_receipt: &'a Receipt,
        contract_id: ContractId,
    ) -> Option<&'a Vec<u8>> {
        match (current_receipt, next_receipt) {
            (
                Receipt::ReturnData {
                    id: first_id,
                    data: first_data,
                    ..
                },
                Receipt::ReturnData {
                    id: second_id,
                    data: vec_data,
                    ..
                },
            ) if *first_id == contract_id
                && !first_data.is_empty()
                && *second_id == ContractId::zeroed() =>
            {
                Some(vec_data)
            }
            _ => None,
        }
    }
}
