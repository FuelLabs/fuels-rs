use fuel_tx::{ContractId, Receipt};
use fuels_core::{
    codec::{ABIDecoder, DecoderConfig},
    types::{
        bech32::Bech32ContractId,
        errors::{error, Error, Result},
        param_types::ParamType,
        Token,
    },
};

pub struct ReceiptParser {
    receipts: Vec<Receipt>,
    decoder: ABIDecoder,
}

impl ReceiptParser {
    pub fn new(receipts: &[Receipt], decoder_config: DecoderConfig) -> Self {
        let relevant_receipts: Vec<Receipt> = receipts
            .iter()
            .filter(|receipt| {
                matches!(receipt, Receipt::ReturnData { .. } | Receipt::Return { .. })
            })
            .cloned()
            .collect();

        Self {
            receipts: relevant_receipts,
            decoder: ABIDecoder::new(decoder_config),
        }
    }

    /// Based on receipts returned by a script transaction, the contract ID (in the case of a contract call),
    /// and the output param, parse the values and return them as Token.
    pub fn parse(
        &mut self,
        contract_id: Option<&Bech32ContractId>,
        output_param: &ParamType,
    ) -> Result<Token> {
        let contract_id = contract_id
            .map(Into::into)
            // During a script execution, the script's contract id is the **null** contract id
            .unwrap_or_else(ContractId::zeroed);

        let data = self
            .extract_return_data(&contract_id)
            .ok_or_else(|| Self::missing_receipts_error(output_param))?;

        self.decoder.decode(output_param, &data)
    }

    fn missing_receipts_error(output_param: &ParamType) -> Error {
        error!(
            Codec,
            "`ReceiptDecoder`: failed to find matching receipts entry for {output_param:?}"
        )
    }

    fn extract_return_data(&mut self, contract_id: &ContractId) -> Option<Vec<u8>> {
        for (index, receipt) in self.receipts.iter_mut().enumerate() {
            if let Receipt::ReturnData {
                id,
                data: Some(data),
                ..
            } = receipt
            {
                if id == contract_id {
                    let data = std::mem::take(data);
                    self.receipts.remove(index);
                    return Some(data);
                }
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use fuel_tx::ScriptExecutionResult;
    use fuels_core::traits::{Parameterize, Tokenizable};

    use super::*;

    const RECEIPT_VAL: u64 = 225;
    const RECEIPT_DATA: &[u8; 3] = &[8, 8, 3];
    const DECODED_DATA: &[u8; 3] = &[8, 8, 3];

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
            data: Some(data.to_vec()),
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
    async fn receipt_parser_filters_receipts() -> Result<()> {
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

        let parser = ReceiptParser::new(&receipts, Default::default());

        assert_eq!(parser.receipts, relevant_receipts);

        Ok(())
    }

    #[tokio::test]
    async fn receipt_parser_empty_receipts() -> Result<()> {
        let receipts = [];
        let output_param = ParamType::Unit;

        let error = ReceiptParser::new(&receipts, Default::default())
            .parse(Default::default(), &output_param)
            .expect_err("should error");

        let expected_error = ReceiptParser::missing_receipts_error(&output_param);
        assert_eq!(error.to_string(), expected_error.to_string());

        Ok(())
    }

    #[tokio::test]
    async fn receipt_parser_extract_return_data() -> Result<()> {
        let expected_receipts = get_relevant_receipts();
        let contract_id = target_contract();

        let mut receipts = expected_receipts.clone();
        receipts.push(get_return_data_receipt(contract_id, RECEIPT_DATA));
        let mut parser = ReceiptParser::new(&receipts, Default::default());

        let token = parser
            .parse(Some(&contract_id.into()), &<[u8; 3]>::param_type())
            .expect("parsing should succeed");

        assert_eq!(&<[u8; 3]>::from_token(token)?, DECODED_DATA);
        assert_eq!(parser.receipts, expected_receipts);

        Ok(())
    }

    #[tokio::test]
    async fn receipt_parser_extract_return() -> Result<()> {
        let expected_receipts = get_relevant_receipts();
        let contract_id = target_contract();

        let mut receipts = expected_receipts.clone();
        receipts.push(get_return_data_receipt(
            contract_id,
            &RECEIPT_VAL.to_be_bytes(),
        ));
        let mut parser = ReceiptParser::new(&receipts, Default::default());

        let token = parser
            .parse(Some(&contract_id.into()), &u64::param_type())
            .expect("parsing should succeed");

        assert_eq!(u64::from_token(token)?, RECEIPT_VAL);
        assert_eq!(parser.receipts, expected_receipts);

        Ok(())
    }
}
