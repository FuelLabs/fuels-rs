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
            Err(Self::decoding_error(output_param))
        }
    }

    fn decoding_error(output_param: &ParamType) -> Error {
        error!(
            InvalidData,
            "ReceiptDecoder: failed to find matching receipts entry for {:?}", output_param
        )
    }

    fn extract_return_data(&mut self, contract_id: ContractId) -> Option<Vec<u8>> {
        let (index, data) = self.receipts.iter().enumerate().find_map(|(i, receipt)| {
            if matches!(receipt,
                    Receipt::ReturnData { id, .. } if *id == contract_id)
            {
                Some((
                    i,
                    receipt
                        .data()
                        .expect("ReturnData should have data")
                        .to_vec(),
                ))
            } else {
                None
            }
        })?;

        self.receipts.remove(index);
        Some(data)
    }

    fn extract_return(&mut self, contract_id: ContractId) -> Option<Vec<u8>> {
        let (index, data) = self.receipts.iter().enumerate().find_map(|(i, receipt)| {
            if matches!(receipt,
                    Receipt::Return { id, ..} if *id == contract_id)
            {
                Some((
                    i,
                    receipt
                        .val()
                        .expect("Return should have val")
                        .to_be_bytes()
                        .to_vec(),
                ))
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
        if let Some((i, vector_data)) = self.receipts.iter().enumerate().tuple_windows().find_map(
            |((i, current_receipt), (_, next_receipt))| {
                let vec_data = Self::extract_vec_data(current_receipt, next_receipt, contract_id)?;
                Some((i, vec_data.clone()))
            },
        ) {
            self.receipts.remove(i);
            // the index has adjusted due to the previous removal
            self.receipts.remove(i);
            Some(vector_data)
        } else {
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

#[cfg(test)]
mod tests {
    use fuel_tx::ScriptExecutionResult;

    use super::*;

    const ENCODED_VAL: u64 = 225;
    const ENCODED_DATA: &[u8; 3] = &[8, 8, 3];

    fn target_contract() -> ContractId {
        ContractId::from([1u8; 32])
    }

    fn get_return_receipt(id: ContractId, val: u64) -> Receipt {
        Receipt::Return {
            id,
            val,
            pc: Default::default(),
            is: Default::default(),
        }
    }

    fn get_return_data_receipt(id: ContractId, data: &[u8]) -> Receipt {
        Receipt::ReturnData {
            id,
            ptr: Default::default(),
            len: Default::default(),
            digest: Default::default(),
            data: data.to_vec(),
            pc: Default::default(),
            is: Default::default(),
        }
    }

    fn get_relevant_receipts() -> Vec<Receipt> {
        vec![
            get_return_receipt(Default::default(), Default::default()),
            get_return_data_receipt(Default::default(), Default::default()),
        ]
    }

    #[tokio::test]
    async fn receipt_decoder_filters_receipts() -> Result<()> {
        let mut receipts = vec![
            Receipt::Call {
                id: Default::default(),
                to: Default::default(),
                amount: Default::default(),
                asset_id: Default::default(),
                gas: Default::default(),
                param1: Default::default(),
                param2: Default::default(),
                pc: Default::default(),
                is: Default::default(),
            },
            Receipt::Revert {
                id: Default::default(),
                ra: Default::default(),
                pc: Default::default(),
                is: Default::default(),
            },
            Receipt::Log {
                id: Default::default(),
                ra: Default::default(),
                rb: Default::default(),
                rc: Default::default(),
                rd: Default::default(),
                pc: Default::default(),
                is: Default::default(),
            },
            Receipt::LogData {
                id: Default::default(),
                ra: Default::default(),
                rb: Default::default(),
                ptr: Default::default(),
                len: Default::default(),
                digest: Default::default(),
                data: Default::default(),
                pc: Default::default(),
                is: Default::default(),
            },
            Receipt::ScriptResult {
                result: ScriptExecutionResult::Success,
                gas_used: Default::default(),
            },
        ];
        let relevant_receipts = get_relevant_receipts();
        receipts.extend(relevant_receipts.clone());

        let decoder = ReceiptDecoder::new(&receipts);

        assert_eq!(decoder.receipts, relevant_receipts);

        Ok(())
    }

    #[tokio::test]
    async fn receipt_decoder_empty_receipts() -> Result<()> {
        let receipts = [];
        let mut decoder = ReceiptDecoder::new(&receipts);
        let output_param = ParamType::Unit;

        let error = decoder
            .try_decode_output(Default::default(), &output_param)
            .expect_err("should error");

        let expected_error = ReceiptDecoder::decoding_error(&output_param);
        assert_eq!(error.to_string(), expected_error.to_string());

        Ok(())
    }

    #[tokio::test]
    async fn receipt_decoder_extract_return_data() -> Result<()> {
        let expected_receipts = get_relevant_receipts();

        let mut receipts = expected_receipts.clone();
        receipts.push(get_return_data_receipt(target_contract(), ENCODED_DATA));
        let mut decoder = ReceiptDecoder::new(&receipts);

        let encoded_data = decoder
            .extract_return_data(target_contract())
            .expect("This should return data");

        assert_eq!(encoded_data, ENCODED_DATA);
        assert_eq!(decoder.receipts, expected_receipts);

        Ok(())
    }

    #[tokio::test]
    async fn receipt_decoder_extract_return() -> Result<()> {
        let expected_receipts = get_relevant_receipts();

        let mut receipts = expected_receipts.clone();
        receipts.push(get_return_receipt(target_contract(), ENCODED_VAL));
        let mut decoder = ReceiptDecoder::new(&receipts);

        let encoded_data = decoder
            .extract_return(target_contract())
            .expect("This should return data");

        assert_eq!(encoded_data, ENCODED_VAL.to_be_bytes().to_vec());
        assert_eq!(decoder.receipts, expected_receipts);

        Ok(())
    }

    #[tokio::test]
    async fn receipt_decoder_extract_return_data_heap() -> Result<()> {
        let expected_receipts = get_relevant_receipts();

        let mut receipts = expected_receipts.clone();
        receipts.push(get_return_data_receipt(target_contract(), &[9, 9, 9]));
        receipts.push(get_return_data_receipt(Default::default(), ENCODED_DATA));
        let mut decoder = ReceiptDecoder::new(&receipts);

        let encoded_data = decoder
            .extract_return_data_heap(target_contract())
            .expect("This should return data");

        assert_eq!(encoded_data, ENCODED_DATA);
        assert_eq!(decoder.receipts, expected_receipts);

        Ok(())
    }
}
