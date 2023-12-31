#![no_main]
use fuels::core::codec::DecoderConfig;
use fuels::prelude::*;
use fuels::programs::receipt_parser::ReceiptParser;
use fuels::tx::Receipt;
use fuels::types::param_types::ParamType;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: (Vec<Receipt>, ParamType)| {
    let contract_id = Bech32ContractId::default();
    let (receipts, param_type) = data;
    let receipts = replace_contract_id_in_receipt(receipts, contract_id.clone().into());
    let decoder_config = DecoderConfig::default();
    let mut receipt_parser = ReceiptParser::new(&receipts, decoder_config);
    let _ = receipt_parser.parse(Some(&contract_id), &param_type);
});

fn replace_contract_id_in_receipt(
    mut receipts: Vec<Receipt>,
    target_contract_id: ContractId,
) -> Vec<Receipt> {
    receipts.iter_mut().for_each(|r| match r {
        Receipt::Call { ref mut id, .. } => *id = target_contract_id,
        Receipt::Return { ref mut id, .. } => *id = target_contract_id,
        Receipt::ReturnData { ref mut id, .. } => *id = target_contract_id,
        Receipt::Panic { ref mut id, .. } => *id = target_contract_id,
        Receipt::Revert { ref mut id, .. } => *id = target_contract_id,
        Receipt::Log { ref mut id, .. } => *id = target_contract_id,
        Receipt::LogData { ref mut id, .. } => *id = target_contract_id,
        Receipt::Transfer { ref mut id, .. } => *id = target_contract_id,
        Receipt::TransferOut { ref mut id, .. } => *id = target_contract_id,
        Receipt::Mint {
            ref mut contract_id,
            ..
        } => *contract_id = target_contract_id,
        Receipt::Burn {
            ref mut contract_id,
            ..
        } => *contract_id = target_contract_id,
        _ => (),
    });
    receipts
}
