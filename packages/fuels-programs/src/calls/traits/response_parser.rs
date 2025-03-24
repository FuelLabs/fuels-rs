use fuel_tx::Receipt;
use fuels_core::{
    codec::DecoderConfig,
    types::{Token, errors::Result, param_types::ParamType},
};

use crate::calls::{ContractCall, ScriptCall, receipt_parser::ReceiptParser, utils::sealed};

pub trait ResponseParser: sealed::Sealed {
    fn parse_call(
        &self,
        receipts: &[Receipt],
        decoder_config: DecoderConfig,
        param_type: &ParamType,
    ) -> Result<Token>;
}

impl ResponseParser for ContractCall {
    fn parse_call(
        &self,
        receipts: &[Receipt],
        decoder_config: DecoderConfig,
        param_type: &ParamType,
    ) -> Result<Token> {
        ReceiptParser::new(receipts, decoder_config).parse_call(&self.contract_id, param_type)
    }
}

impl ResponseParser for ScriptCall {
    fn parse_call(
        &self,
        receipts: &[Receipt],
        decoder_config: DecoderConfig,
        param_type: &ParamType,
    ) -> Result<Token> {
        ReceiptParser::new(receipts, decoder_config).parse_script(param_type)
    }
}
