use fuel_asm::{Instruction, Opcode};
use fuel_tx::{AssetId, ContractId};
use fuels_core::{
    constants::WORD_SIZE,
    error,
    types::{errors::Result, transaction_builders::BlobId},
};
use itertools::Itertools;

use crate::{calls::ContractCallData, executable::loader_instructions, utils::prepend_msg};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptDescription {
    pub code: Vec<u8>,
    pub data_section_offset: Option<u64>,
    pub data: Vec<u8>,
}

struct WasmFriendlyCursor<'a> {
    data: &'a [u8],
}

impl<'a> WasmFriendlyCursor<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data }
    }

    pub fn consume(&mut self, amount: usize, ctx: &'static str) -> Result<&'a [u8]> {
        if self.data.len() < amount {
            Err(error!(
                Other,
                "while decoding {ctx}: not enough data, available: {}, requested: {}",
                self.data.len(),
                amount
            ))
        } else {
            let data = &self.data[..amount];
            self.data = &self.data[amount..];
            Ok(data)
        }
    }

    pub fn consume_all(&self) -> &'a [u8] {
        self.data
    }

    pub fn unconsumed(&self) -> usize {
        self.data.len()
    }
}

impl ScriptDescription {
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
    Loader(ScriptDescription, BlobId),
    Other(ScriptDescription),
}

struct ContractCallInstructions {
    instructions: Vec<Instruction>,
}

impl ContractCallInstructions {
    pub fn new(instructions: &[Instruction]) -> Option<(Self, usize)> {
        let gas_fwd = Self::check_gas_fwd_variant(instructions);
        let normal = Self::check_normal_variant(instructions);
        if gas_fwd || normal {
            let num_instructions = if gas_fwd {
                Self::GAS_FWD_OPCODES.len()
            } else {
                Self::NO_GAS_FWD_OPCODES.len()
            };

            let instructions: Vec<_> = instructions
                .iter()
                .take(num_instructions)
                .cloned()
                .collect();
            let num_instructions_taken = instructions.len();

            Some((Self { instructions }, num_instructions_taken))
        } else {
            None
        }
    }
    fn call_data_offset(&self) -> u32 {
        let Instruction::MOVI(movi) = self.instructions[0] else {
            panic!("should have validated the first instruction is a MOVI");
        };

        movi.imm18().into()
    }

    fn describe_contract_call(&self, script_data: &[u8]) -> Result<ContractCallData> {
        let mut data = WasmFriendlyCursor::new(script_data);

        let amount = u64::from_be_bytes(
            data.consume(8, "amount")?
                .try_into()
                .expect("will have exactly 8 bytes"),
        );

        let asset_id = AssetId::new(
            data.consume(32, "asset id")?
                .try_into()
                .expect("will have exactly 32 bytes"),
        );

        let contract_id = ContractId::new(
            data.consume(32, "contract id")?
                .try_into()
                .expect("will have exactly 32 bytes"),
        );

        let _ = data.consume(8, "function selector offset")?;

        let _ = data.consume(8, "encoded args offset")?;

        let fn_selector = {
            let fn_selector_len = {
                let bytes = data.consume(8, "function selector lenght")?;
                u64::from_be_bytes(bytes.try_into().expect("will have exactly 8 bytes")) as usize
            };
            data.consume(fn_selector_len, "function selector")?.to_vec()
        };

        let (encoded_args, gas_forwarded) = if self.has_gas_forwarding_instructions() {
            let encoded_args = data
                .consume(data.unconsumed().saturating_sub(WORD_SIZE), "encoded_args")?
                .to_vec();

            let gas_fwd = {
                let gas_fwd_bytes = data.consume(WORD_SIZE, "forwarded gas")?;
                u64::from_be_bytes(gas_fwd_bytes.try_into().expect("exactly 8 bytes"))
            };

            (encoded_args, Some(gas_fwd))
        } else {
            (data.consume_all().to_vec(), None)
        };

        Ok(ContractCallData {
            amount,
            asset_id,
            contract_id,
            fn_selector_encoded: fn_selector,
            encoded_args,
            gas_forwarded,
        })
    }

    fn has_gas_forwarding_instructions(&self) -> bool {
        Self::check_gas_fwd_variant(&self.instructions)
    }

    const NO_GAS_FWD_OPCODES: [Opcode; 5] = [
        Opcode::MOVI,
        Opcode::MOVI,
        Opcode::LW,
        Opcode::MOVI,
        Opcode::CALL,
    ];

    const GAS_FWD_OPCODES: [Opcode; 7] = [
        Opcode::MOVI,
        Opcode::MOVI,
        Opcode::LW,
        Opcode::MOVI,
        Opcode::MOVI,
        Opcode::LW,
        Opcode::CALL,
    ];

    fn check_normal_variant(instructions: &[Instruction]) -> bool {
        if instructions.len() < Self::NO_GAS_FWD_OPCODES.len() {
            return false;
        }

        Self::NO_GAS_FWD_OPCODES
            .iter()
            .zip(instructions.iter())
            .all(|(expected, actual)| expected == &actual.opcode())
    }

    fn check_gas_fwd_variant(instructions: &[Instruction]) -> bool {
        if instructions.len() < Self::GAS_FWD_OPCODES.len() {
            return false;
        }

        Self::GAS_FWD_OPCODES
            .iter()
            .zip(instructions.iter())
            .all(|(expected, actual)| expected == &actual.opcode())
    }
}

fn parse_script_call(script: &[u8], script_data: &[u8]) -> Option<ScriptDescription> {
    let data_section_offset = if script.len() >= 16 {
        let data_offset = u64::from_be_bytes(script[8..16].try_into().unwrap());
        if data_offset as usize >= script.len() {
            None
        } else {
            Some(data_offset)
        }
    } else {
        None
    };

    Some(ScriptDescription {
        data: script_data.to_vec(),
        data_section_offset,
        code: script.to_vec(),
    })
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

    let mut instructions = instructions.as_slice();

    let mut call_instructions = vec![];

    while let Some((parsed_instructions, amount_read)) = ContractCallInstructions::new(instructions)
    {
        debug_assert!(amount_read > 0);
        instructions = &instructions[amount_read..];
        call_instructions.push(parsed_instructions);
    }

    if !instructions.is_empty() {
        match instructions {
            [single_instruction] if single_instruction.opcode() == Opcode::RET => {
                eprintln!("Single instruction is a RET, that's fine")
            }
            _ => {
                return Ok(None);
            }
        }
    }

    let Some(minimum_call_offset) = call_instructions.iter().map(|i| i.call_data_offset()).min()
    else {
        return Ok(None);
    };

    let mut descriptions = vec![];
    let num_calls = call_instructions.len();

    for (idx, current_call_instructions) in call_instructions.iter().enumerate() {
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

        let contract_call_description =
            current_call_instructions.describe_contract_call(&script_data[data_start..data_end])?;

        descriptions.push(contract_call_description);
    }

    Ok(Some(descriptions))
}

pub fn parse_script(script: &[u8], data: &[u8]) -> Result<ScriptType> {
    if let Some(contract_calls) =
        parse_contract_calls(script, data).map_err(prepend_msg("while decoding contract call"))?
    {
        return Ok(ScriptType::ContractCall(contract_calls));
    }

    if let Some((script, blob_id)) =
        parse_loader_script(script, data).map_err(prepend_msg("while decoding loader script"))?
    {
        return Ok(ScriptType::Loader(script, blob_id));
    }

    if let Some(script) = parse_script_call(script, data) {
        return Ok(ScriptType::Other(script));
    }

    unimplemented!()
}

fn parse_loader_script(
    script: &[u8],
    data: &[u8],
) -> Result<Option<(ScriptDescription, [u8; 32])>> {
    // TODO: handle no data section
    let expected_loader_instructions = loader_instructions();
    let loader_instructions_byte_size = expected_loader_instructions.len() * Instruction::SIZE;

    let mut script_cursor = WasmFriendlyCursor::new(script);

    // replace with split_at_checked when we move to msrv 1.80.0
    if script_cursor.unconsumed() < loader_instructions_byte_size {
        return Ok(None);
    }

    let instructions: std::result::Result<Vec<Instruction>, _> = fuel_asm::from_bytes(
        script_cursor
            .consume(loader_instructions_byte_size, "loader instructions")
            .expect("will have enough bytes")
            .to_vec(),
    )
    .try_collect();

    let Ok(instructions) = instructions else {
        return Ok(None);
    };

    if instructions
        .iter()
        .zip(expected_loader_instructions.iter())
        .any(|(actual, expected)| actual != expected)
    {
        return Ok(None);
    }

    let blob_id = script_cursor
        .consume(32, "blob id")?
        .try_into()
        .expect("will have exactly 32 bytes");

    let _data_section_len = script_cursor.consume(WORD_SIZE, "data section len")?;

    Ok(Some((
        ScriptDescription {
            code: script.to_vec(),
            data: data.to_vec(),
            data_section_offset: Some(
                script.len().saturating_sub(script_cursor.unconsumed()) as u64
            ),
        },
        blob_id,
    )))
}

#[cfg(test)]
mod tests {
    use fuel_asm::RegId;
    use fuels_core::types::errors::Error;
    use rand::{RngCore, SeedableRng};
    use test_case::test_case;

    use crate::calls::utils::{get_single_call_instructions, CallOpcodeParamsOffset};

    use super::*;

    #[test]
    fn can_handle_empty_scripts() {
        // given
        let empty_script = [];

        // when
        let res = parse_script(&empty_script, &[]).unwrap();

        // then
        assert_eq!(
            res,
            ScriptType::Other(ScriptDescription {
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
        let script_type = parse_script(&script, &[]).unwrap();

        // then
        assert_eq!(
            script_type,
            ScriptType::Other(ScriptDescription {
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
        let script_type = parse_script(&handwritten_script, &[]).unwrap();

        // then
        assert_eq!(
            script_type,
            ScriptType::Other(ScriptDescription {
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
    #[test_case(20, "function selector lenght")]
    #[test_case(12, "function selector")]
    #[test_case(8, "forwarded gas")]
    fn catches_missing_data(amount_of_data_to_steal: usize, expected_msg: &str) {
        // given
        let script = get_single_call_instructions(&CallOpcodeParamsOffset {
            call_data_offset: 0,
            amount_offset: 0,
            asset_id_offset: 0,
            gas_forwarded_offset: Some(1),
        })
        .unwrap();

        let ok_data = example_contract_call_data(false, true);
        let not_enough_data = ok_data[..ok_data.len() - amount_of_data_to_steal].to_vec();

        // when
        let err = parse_script(&script, &not_enough_data).unwrap_err();

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
        let script = get_single_call_instructions(&CallOpcodeParamsOffset {
            call_data_offset: 0,
            amount_offset: 0,
            asset_id_offset: 0,
            gas_forwarded_offset: Some(1),
        })
        .unwrap();

        let invalid_utf8 = {
            let invalid_data = [0x80, 0xBF, 0xC0, 0xAF, 0xFF];
            assert!(String::from_utf8(invalid_data.to_vec()).is_err());
            invalid_data
        };

        let mut ok_data = example_contract_call_data(false, true);
        ok_data[96..101].copy_from_slice(&invalid_utf8);

        // when
        let script_type = parse_script(&script, &ok_data).unwrap();

        // then
        let ScriptType::ContractCall(datas) = script_type else {
            panic!("expected ScriptType::Other");
        };
        let Error::Codec(err) = datas[0].decode_fn_selector().unwrap_err() else {
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
        let script = loader_instructions()
            .iter()
            .flat_map(|i| i.to_bytes())
            .collect::<Vec<_>>();

        // when
        let err = parse_script(&script, &[]).unwrap_err();

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
        let mut loader_instructions = loader_instructions().to_vec();

        loader_instructions.insert(
            loader_instructions.len() - 2,
            fuel_asm::op::movi(RegId::ZERO, 0),
        );
        let script = loader_instructions
            .iter()
            .flat_map(|i| i.to_bytes())
            .collect::<Vec<_>>();

        // when
        let script_type = parse_script(&script, &[]).unwrap();

        // then
        assert_eq!(
            script_type,
            ScriptType::Other(ScriptDescription {
                code: script,
                data_section_offset: None,
                data: vec![]
            })
        );
    }

    #[test]
    fn extra_instructions_in_contract_calling_scripts_not_tolerated() {
        // given
        let mut contract_call_script = get_single_call_instructions(&CallOpcodeParamsOffset {
            call_data_offset: 0,
            amount_offset: 0,
            asset_id_offset: 0,
            gas_forwarded_offset: Some(1),
        })
        .unwrap();
        contract_call_script.extend(fuel_asm::op::movi(RegId::ZERO, 10).to_bytes());
        let script_data = example_contract_call_data(false, true);

        // when
        let script_type = parse_script(&contract_call_script, &script_data).unwrap();

        // then
        assert_eq!(
            script_type,
            ScriptType::Other(ScriptDescription {
                code: contract_call_script,
                data_section_offset: None,
                data: script_data
            })
        );
    }

    #[test]
    fn handles_invalid_call_data_offset() {
        // given
        let contract_call_1 = get_single_call_instructions(&CallOpcodeParamsOffset {
            call_data_offset: 0,
            amount_offset: 0,
            asset_id_offset: 0,
            gas_forwarded_offset: Some(1),
        })
        .unwrap();

        let contract_call_2 = get_single_call_instructions(&CallOpcodeParamsOffset {
            call_data_offset: u16::MAX as usize,
            amount_offset: 0,
            asset_id_offset: 0,
            gas_forwarded_offset: Some(1),
        })
        .unwrap();

        let data_only_for_one_call = example_contract_call_data(false, true);

        let together = [&contract_call_1, &contract_call_2]
            .iter()
            .flat_map(|i| i.iter().cloned())
            .collect::<Vec<_>>();

        // when
        let err = parse_script(&together, &data_only_for_one_call).unwrap_err();

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
