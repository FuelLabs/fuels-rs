use fuel_tx::Receipt;
use fuels_core::{
    codec::DecoderConfig,
    types::{errors::Result, param_types::ParamType, Token},
};

use crate::calls::{receipt_parser::ReceiptParser, utils::sealed, ContractCall, ScriptCall};

pub trait Parsable: sealed::Sealed {
    fn parse_call(
        &self,
        receipts: &[Receipt],
        decoder_config: DecoderConfig,
        param_type: &ParamType,
    ) -> Result<Token>;
}

impl Parsable for ContractCall {
    fn parse_call(
        &self,
        receipts: &[Receipt],
        decoder_config: DecoderConfig,
        param_type: &ParamType,
    ) -> Result<Token> {
        ReceiptParser::new(receipts, decoder_config).parse_call(&self.contract_id, param_type)
    }
}

impl Parsable for ScriptCall {
    fn parse_call(
        &self,
        receipts: &[Receipt],
        decoder_config: DecoderConfig,
        param_type: &ParamType,
    ) -> Result<Token> {
        ReceiptParser::new(receipts, decoder_config).parse_script(param_type)
    }
}
