use std::collections::VecDeque;

use fuel_tx::Receipt;
use fuels_core::{
    codec::{ABIDecoder, DecoderConfig},
    types::{
        ContractId, Token,
        errors::{Error, Result, error},
        param_types::ParamType,
    },
};

pub struct ReceiptParser {
    receipts: VecDeque<Receipt>,
    decoder: ABIDecoder,
}

impl ReceiptParser {
    pub fn new(receipts: &[Receipt], decoder_config: DecoderConfig) -> Self {
        let relevant_receipts = receipts
            .iter()
            .filter(|receipt| matches!(receipt, Receipt::ReturnData { .. } | Receipt::Call { .. }))
            .cloned()
            .collect();

        Self {
            receipts: relevant_receipts,
            decoder: ABIDecoder::new(decoder_config),
        }
    }

    /// Based on receipts returned by a script transaction, the contract ID,
    /// and the output param, parse the values and return them as Token.
    pub fn parse_call(
        &mut self,
        contract_id: ContractId,
        output_param: &ParamType,
    ) -> Result<Token> {
        let data = self
            .extract_contract_call_data(contract_id)
            .ok_or_else(|| Self::missing_receipts_error(output_param))?;

        self.decoder.decode(output_param, data.as_slice())
    }

    pub fn parse_script(self, output_param: &ParamType) -> Result<Token> {
        let data = self
            .extract_script_data()
            .ok_or_else(|| Self::missing_receipts_error(output_param))?;

        self.decoder.decode(output_param, data.as_slice())
    }

    fn missing_receipts_error(output_param: &ParamType) -> Error {
        error!(
            Codec,
            "`ReceiptDecoder`: failed to find matching receipts entry for {output_param:?}"
        )
    }

    pub fn extract_contract_call_data(&mut self, target_contract: ContractId) -> Option<Vec<u8>> {
        // If the script contains nested calls, we need to extract the data of the top-level call
        let mut nested_calls_stack = vec![];

        while let Some(receipt) = self.receipts.pop_front() {
            if let Receipt::Call { to, .. } = receipt {
                nested_calls_stack.push(to);
            } else if let Receipt::ReturnData {
                data,
                id: return_id,
                ..
            } = receipt
            {
                let call_id = nested_calls_stack.pop();

                // Somethings off if there is a mismatch between the call and return ids
                debug_assert_eq!(call_id.unwrap(), return_id);

                if nested_calls_stack.is_empty() {
                    // The top-level call return should match our target contract
                    debug_assert_eq!(target_contract, return_id);

                    return data.clone();
                }
            }
        }

        None
    }

    fn extract_script_data(&self) -> Option<Vec<u8>> {
        self.receipts.iter().find_map(|receipt| match receipt {
            Receipt::ReturnData {
                id,
                data: Some(data),
                ..
            } if *id == ContractId::zeroed() => Some(data.clone()),
            _ => None,
        })
    }
}

#[cfg(test)]
mod tests {
    use fuel_tx::ScriptExecutionResult;
    use fuels_core::traits::{Parameterize, Tokenizable};

    use super::*;

    const RECEIPT_DATA: &[u8; 3] = &[8, 8, 3];
    const DECODED_DATA: &[u8; 3] = &[8, 8, 3];

    fn target_contract() -> ContractId {
        ContractId::from([1u8; 32])
    }

    fn get_return_data_receipt(id: ContractId, data: &[u8]) -> Receipt {
        Receipt::ReturnData {
            id,
            ptr: Default::default(),
            len: Default::default(),
            digest: Default::default(),
            data: Some(data.to_vec()),
            pc: Default::default(),
            is: Default::default(),
        }
    }

    fn get_call_receipt(to: ContractId) -> Receipt {
        Receipt::Call {
            id: Default::default(),
            to,
            amount: Default::default(),
            asset_id: Default::default(),
            gas: Default::default(),
            param1: Default::default(),
            param2: Default::default(),
            pc: Default::default(),
            is: Default::default(),
        }
    }

    fn get_relevant_receipts() -> Vec<Receipt> {
        let id = target_contract();
        vec![
            get_call_receipt(id),
            get_return_data_receipt(id, RECEIPT_DATA),
        ]
    }

    #[tokio::test]
    async fn receipt_parser_filters_receipts() -> Result<()> {
        let mut receipts = vec![
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

        let parser = ReceiptParser::new(&receipts, Default::default());

        assert_eq!(parser.receipts, relevant_receipts);

        Ok(())
    }

    #[tokio::test]
    async fn receipt_parser_empty_receipts() -> Result<()> {
        let receipts = [];
        let output_param = ParamType::U8;

        let error = ReceiptParser::new(&receipts, Default::default())
            .parse_call(target_contract(), &output_param)
            .expect_err("should error");

        let expected_error = ReceiptParser::missing_receipts_error(&output_param);
        assert_eq!(error.to_string(), expected_error.to_string());

        Ok(())
    }

    #[tokio::test]
    async fn receipt_parser_extract_return_data() -> Result<()> {
        let receipts = get_relevant_receipts();

        let mut parser = ReceiptParser::new(&receipts, Default::default());

        let token = parser
            .parse_call(target_contract(), &<[u8; 3]>::param_type())
            .expect("parsing should succeed");

        assert_eq!(&<[u8; 3]>::from_token(token)?, DECODED_DATA);

        Ok(())
    }

    #[tokio::test]
    async fn receipt_parser_extracts_top_level_call_receipts() -> Result<()> {
        const CORRECT_DATA_1: [u8; 3] = [1, 2, 3];
        const CORRECT_DATA_2: [u8; 3] = [5, 6, 7];

        let contract_top_lvl = target_contract();
        let contract_nested = ContractId::from([9u8; 32]);

        let receipts = vec![
            get_call_receipt(contract_top_lvl),
            get_call_receipt(contract_nested),
            get_return_data_receipt(contract_nested, &[9, 9, 9]),
            get_return_data_receipt(contract_top_lvl, &CORRECT_DATA_1),
            get_call_receipt(contract_top_lvl),
            get_call_receipt(contract_nested),
            get_return_data_receipt(contract_nested, &[7, 7, 7]),
            get_return_data_receipt(contract_top_lvl, &CORRECT_DATA_2),
        ];

        let mut parser = ReceiptParser::new(&receipts, Default::default());

        let token_1 = parser
            .parse_call(contract_top_lvl, &<[u8; 3]>::param_type())
            .expect("parsing should succeed");
        let token_2 = parser
            .parse_call(contract_top_lvl, &<[u8; 3]>::param_type())
            .expect("parsing should succeed");

        assert_eq!(&<[u8; 3]>::from_token(token_1)?, &CORRECT_DATA_1);
        assert_eq!(&<[u8; 3]>::from_token(token_2)?, &CORRECT_DATA_2);

        Ok(())
    }
}
