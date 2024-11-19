use fuel_asm::{Instruction, Opcode};
use fuels_core::{error, types::errors::Result};
use itertools::Itertools;

use crate::{
    assembly::{
        contract_call::{ContractCallData, ContractCallInstructions},
        script_and_predicate_loader::LoaderCode,
    },
    utils::prepend_msg,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptCallData {
    pub code: Vec<u8>,
    pub data_section_offset: Option<u64>,
    pub data: Vec<u8>,
}

impl ScriptCallData {
    pub fn data_section(&self) -> Option<&[u8]> {
        self.data_section_offset.map(|offset| {
            let offset = offset as usize;
            &self.code[offset..]
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScriptType {
    ContractCall(Vec<ContractCallData>),
    Loader {
        script: ScriptCallData,
        blob_id: [u8; 32],
    },
    Other(ScriptCallData),
}

fn parse_script_call(script: &[u8], script_data: &[u8]) -> ScriptCallData {
    let data_section_offset = if script.len() >= 16 {
        let data_offset = u64::from_be_bytes(script[8..16].try_into().expect("will have 8 bytes"));
        if data_offset as usize >= script.len() {
            None
        } else {
            Some(data_offset)
        }
    } else {
        None
    };

    ScriptCallData {
        data: script_data.to_vec(),
        data_section_offset,
        code: script.to_vec(),
    }
}

fn parse_contract_calls(
    script: &[u8],
    script_data: &[u8],
) -> Result<Option<Vec<ContractCallData>>> {
    let instructions: std::result::Result<Vec<Instruction>, _> =
        fuel_asm::from_bytes(script.to_vec()).try_collect();

    let Ok(instructions) = instructions else {
        return Ok(None);
    };

    let Some(call_instructions) = extract_call_instructions(&instructions) else {
        return Ok(None);
    };

    let Some(minimum_call_offset) = call_instructions.iter().map(|i| i.call_data_offset()).min()
    else {
        return Ok(None);
    };

    let num_calls = call_instructions.len();

    call_instructions.iter().enumerate().map(|(idx, current_call_instructions)| {
            let data_start =
                (current_call_instructions.call_data_offset() - minimum_call_offset) as usize;

            let data_end = if idx + 1 < num_calls {
                (call_instructions[idx + 1].call_data_offset()
                    - current_call_instructions.call_data_offset()) as usize
            } else {
                script_data.len()
            };

            if data_start > script_data.len() || data_end > script_data.len() {
                return Err(error!(
                    Other,
                    "call data offset requires data section of length {}, but data section is only {} bytes long",
                    data_end,
                    script_data.len()
                ));
            }

            let contract_call_data = ContractCallData::decode(
                &script_data[data_start..data_end],
                current_call_instructions.is_gas_fwd_variant(),
            )?;

            Ok(contract_call_data)
        }).collect::<Result<_>>().map(Some)
}

fn extract_call_instructions(
    mut instructions: &[Instruction],
) -> Option<Vec<ContractCallInstructions>> {
    let mut call_instructions = vec![];

    while let Some(extracted_instructions) = ContractCallInstructions::extract_from(instructions) {
        let num_instructions = extracted_instructions.len();
        debug_assert!(num_instructions > 0);

        instructions = &instructions[num_instructions..];
        call_instructions.push(extracted_instructions);
    }

    if !instructions.is_empty() {
        match instructions {
            [single_instruction] if single_instruction.opcode() == Opcode::RET => {}
            _ => return None,
        }
    }

    Some(call_instructions)
}

impl ScriptType {
    pub fn detect(script: &[u8], data: &[u8]) -> Result<Self> {
        if let Some(contract_calls) = parse_contract_calls(script, data)
            .map_err(prepend_msg("while decoding contract call"))?
        {
            return Ok(Self::ContractCall(contract_calls));
        }

        if let Some((script, blob_id)) = parse_loader_script(script, data)? {
            return Ok(Self::Loader { script, blob_id });
        }

        Ok(Self::Other(parse_script_call(script, data)))
    }
}

fn parse_loader_script(script: &[u8], data: &[u8]) -> Result<Option<(ScriptCallData, [u8; 32])>> {
    let Some(loader_code) = LoaderCode::from_loader_binary(script)
        .map_err(prepend_msg("while decoding loader script"))?
    else {
        return Ok(None);
    };

    Ok(Some((
        ScriptCallData {
            code: script.to_vec(),
            data: data.to_vec(),
            data_section_offset: Some(loader_code.data_section_offset() as u64),
        },
        loader_code.blob_id(),
    )))
}

#[cfg(test)]
mod tests {

    use fuel_asm::RegId;
    use fuels_core::types::errors::Error;
    use rand::{RngCore, SeedableRng};
    use test_case::test_case;

    use crate::assembly::{
        contract_call::{CallOpcodeParamsOffset, ContractCallInstructions},
        script_and_predicate_loader::loader_instructions_w_data_section,
    };

    use super::*;

    #[test]
    fn can_handle_empty_scripts() {
        // given
        let empty_script = [];

        // when
        let res = ScriptType::detect(&empty_script, &[]).unwrap();

        // then
        assert_eq!(
            res,
            ScriptType::Other(ScriptCallData {
                code: vec![],
                data_section_offset: None,
                data: vec![]
            })
        )
    }

    #[test]
    fn is_fine_with_malformed_scripts() {
        // given
        let mut script = vec![0; 100 * Instruction::SIZE];
        let mut rng = rand::rngs::StdRng::from_seed([0; 32]);
        rng.fill_bytes(&mut script);

        // when
        let script_type = ScriptType::detect(&script, &[]).unwrap();

        // then
        assert_eq!(
            script_type,
            ScriptType::Other(ScriptCallData {
                code: script,
                data_section_offset: None,
                data: vec![]
            })
        );
    }

    // Mostly to do with the script binary not having the script data offset in the second word
    #[test]
    fn is_fine_with_handwritten_scripts() {
        // given
        let handwritten_script = [
            fuel_asm::op::movi(0x10, 100),
            fuel_asm::op::movi(0x10, 100),
            fuel_asm::op::movi(0x10, 100),
            fuel_asm::op::movi(0x10, 100),
            fuel_asm::op::movi(0x10, 100),
        ]
        .iter()
        .flat_map(|i| i.to_bytes())
        .collect::<Vec<_>>();

        // when
        let script_type = ScriptType::detect(&handwritten_script, &[]).unwrap();

        // then
        assert_eq!(
            script_type,
            ScriptType::Other(ScriptCallData {
                code: handwritten_script.to_vec(),
                data_section_offset: None,
                data: vec![]
            })
        );
    }

    fn example_contract_call_data(has_args: bool, gas_fwd: bool) -> Vec<u8> {
        let mut data = vec![];
        data.extend_from_slice(&100u64.to_be_bytes());
        data.extend_from_slice(&[0; 32]);
        data.extend_from_slice(&[1; 32]);
        data.extend_from_slice(&[0; 8]);
        data.extend_from_slice(&[0; 8]);
        data.extend_from_slice(&"test".len().to_be_bytes());
        data.extend_from_slice("test".as_bytes());
        if has_args {
            data.extend_from_slice(&[0; 8]);
        }
        if gas_fwd {
            data.extend_from_slice(&[0; 8]);
        }
        data
    }

    #[test_case(108, "amount")]
    #[test_case(100, "asset id")]
    #[test_case(68, "contract id")]
    #[test_case(36, "function selector offset")]
    #[test_case(28, "encoded args offset")]
    #[test_case(20, "function selector length")]
    #[test_case(12, "function selector")]
    #[test_case(8, "forwarded gas")]
    fn catches_missing_data(amount_of_data_to_steal: usize, expected_msg: &str) {
        // given
        let script = ContractCallInstructions::new(CallOpcodeParamsOffset {
            call_data_offset: 0,
            amount_offset: 0,
            asset_id_offset: 0,
            gas_forwarded_offset: Some(1),
        })
        .into_bytes()
        .collect_vec();

        let ok_data = example_contract_call_data(false, true);
        let not_enough_data = ok_data[..ok_data.len() - amount_of_data_to_steal].to_vec();

        // when
        let err = ScriptType::detect(&script, &not_enough_data).unwrap_err();

        // then
        let Error::Other(mut msg) = err else {
            panic!("expected Error::Other");
        };

        let expected_msg =
            format!("while decoding contract call: while decoding {expected_msg}: not enough data");
        msg.truncate(expected_msg.len());

        assert_eq!(expected_msg, msg);
    }

    #[test]
    fn handles_invalid_utf8_fn_selector() {
        // given
        let script = ContractCallInstructions::new(CallOpcodeParamsOffset {
            call_data_offset: 0,
            amount_offset: 0,
            asset_id_offset: 0,
            gas_forwarded_offset: Some(1),
        })
        .into_bytes()
        .collect_vec();

        let invalid_utf8 = {
            let invalid_data = [0x80, 0xBF, 0xC0, 0xAF, 0xFF];
            assert!(String::from_utf8(invalid_data.to_vec()).is_err());
            invalid_data
        };

        let mut ok_data = example_contract_call_data(false, true);
        ok_data[96..101].copy_from_slice(&invalid_utf8);

        // when
        let script_type = ScriptType::detect(&script, &ok_data).unwrap();

        // then
        let ScriptType::ContractCall(calls) = script_type else {
            panic!("expected ScriptType::Other");
        };
        let Error::Codec(err) = calls[0].decode_fn_selector().unwrap_err() else {
            panic!("expected Error::Codec");
        };

        assert_eq!(
            err,
            "cannot decode function selector: invalid utf-8 sequence of 1 bytes from index 0"
        );
    }

    #[test]
    fn loader_script_without_a_blob() {
        // given
        let script = loader_instructions_w_data_section()
            .iter()
            .flat_map(|i| i.to_bytes())
            .collect::<Vec<_>>();

        // when
        let err = ScriptType::detect(&script, &[]).unwrap_err();

        // then
        let Error::Other(msg) = err else {
            panic!("expected Error::Other");
        };
        assert_eq!(
            "while decoding loader script: while decoding blob id: not enough data, available: 0, requested: 32",
            msg
        );
    }

    #[test]
    fn loader_script_with_almost_matching_instructions() {
        // given
        let mut loader_instructions = loader_instructions_w_data_section().to_vec();

        loader_instructions.insert(
            loader_instructions.len() - 2,
            fuel_asm::op::movi(RegId::ZERO, 0),
        );
        let script = loader_instructions
            .iter()
            .flat_map(|i| i.to_bytes())
            .collect::<Vec<_>>();

        // when
        let script_type = ScriptType::detect(&script, &[]).unwrap();

        // then
        assert_eq!(
            script_type,
            ScriptType::Other(ScriptCallData {
                code: script,
                data_section_offset: None,
                data: vec![]
            })
        );
    }

    #[test]
    fn extra_instructions_in_contract_calling_scripts_not_tolerated() {
        // given
        let mut contract_call_script = ContractCallInstructions::new(CallOpcodeParamsOffset {
            call_data_offset: 0,
            amount_offset: 0,
            asset_id_offset: 0,
            gas_forwarded_offset: Some(1),
        })
        .into_bytes()
        .collect_vec();

        contract_call_script.extend(fuel_asm::op::movi(RegId::ZERO, 10).to_bytes());
        let script_data = example_contract_call_data(false, true);

        // when
        let script_type = ScriptType::detect(&contract_call_script, &script_data).unwrap();

        // then
        assert_eq!(
            script_type,
            ScriptType::Other(ScriptCallData {
                code: contract_call_script,
                data_section_offset: None,
                data: script_data
            })
        );
    }

    #[test]
    fn handles_invalid_call_data_offset() {
        // given
        let contract_call_1 = ContractCallInstructions::new(CallOpcodeParamsOffset {
            call_data_offset: 0,
            amount_offset: 0,
            asset_id_offset: 0,
            gas_forwarded_offset: Some(1),
        })
        .into_bytes();

        let contract_call_2 = ContractCallInstructions::new(CallOpcodeParamsOffset {
            call_data_offset: u16::MAX as usize,
            amount_offset: 0,
            asset_id_offset: 0,
            gas_forwarded_offset: Some(1),
        })
        .into_bytes();

        let data_only_for_one_call = example_contract_call_data(false, true);

        let together = contract_call_1.chain(contract_call_2).collect_vec();

        // when
        let err = ScriptType::detect(&together, &data_only_for_one_call).unwrap_err();

        // then
        let Error::Other(msg) = err else {
            panic!("expected Error::Other");
        };

        assert_eq!(
            "while decoding contract call: call data offset requires data section of length 65535, but data section is only 108 bytes long",
            msg
        );
    }
}
