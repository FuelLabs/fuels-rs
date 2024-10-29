use fuel_asm::{Instruction, Opcode};
use fuel_tx::{AssetId, ContractId};
use fuels_core::{
    constants::WORD_SIZE,
    types::{errors::Result, transaction_builders::BlobId},
};
use itertools::Itertools;

use crate::executable::loader_instructions;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractCallDescription {
    pub amount: u64,
    pub asset_id: AssetId,
    pub contract_id: ContractId,
    pub fn_selector: String,
    pub encoded_args: Vec<u8>,
    pub gas_forwarded: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptDescription {
    pub code: Vec<u8>,
    pub data_section_offset: Option<u64>,
    pub data: Vec<u8>,
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
    ContractCall(Vec<ContractCallDescription>),
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

    fn describe_contract_call(&self, script_data: &[u8]) -> Result<ContractCallDescription> {
        let amount = u64::from_be_bytes(script_data[..8].try_into().unwrap());
        let data = &script_data[8..];

        let asset_id = AssetId::new(data[..32].try_into().unwrap());
        let data = &data[32..];

        let contract_id = ContractId::new(data[..32].try_into().unwrap());
        let data = &data[32..];

        let _fn_selector_offset = &data[..8];
        let data = &data[8..];

        let _encoded_args_offset = &data[..8];
        let data = &data[8..];

        let fn_selector_len = u64::from_be_bytes(data[..8].try_into().unwrap()) as usize;
        let data = &data[8..];

        let fn_selector = String::from_utf8(data[..fn_selector_len].to_vec()).unwrap();
        let data = &data[fn_selector_len..];

        let encoded_args = if self.has_gas_forwarding_instructions() {
            data[..data.len() - WORD_SIZE].to_vec()
        } else {
            data.to_vec()
        };
        let data = &data[encoded_args.len()..];

        let gas_forwarded = self
            .has_gas_forwarding_instructions()
            .then(|| u64::from_be_bytes(data[..WORD_SIZE].try_into().unwrap()));

        Ok(ContractCallDescription {
            amount,
            asset_id,
            contract_id,
            fn_selector,
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
        Self::NO_GAS_FWD_OPCODES
            .iter()
            .zip(instructions.iter())
            .all(|(expected, actual)| expected == &actual.opcode())
    }

    fn check_gas_fwd_variant(instructions: &[Instruction]) -> bool {
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
) -> Result<Option<Vec<ContractCallDescription>>> {
    let instructions: std::result::Result<Vec<Instruction>, _> =
        fuel_asm::from_bytes(script.to_vec()).try_collect();

    let Ok(instructions) = instructions else {
        return Ok(None);
    };

    let mut instructions = instructions.as_slice();

    let mut call_instructions = vec![];

    while !instructions.is_empty() {
        match instructions {
            [single_instruction] if single_instruction.opcode() == Opcode::RET => break,
            _ => {}
        }

        let Some((parsed_instructions, amount_read)) = ContractCallInstructions::new(instructions)
        else {
            break;
        };
        instructions = &instructions[amount_read..];
        call_instructions.push(parsed_instructions);
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

        let contract_call_description =
            current_call_instructions.describe_contract_call(&script_data[data_start..data_end])?;
        descriptions.push(contract_call_description);
    }

    Ok(Some(descriptions))
}

pub fn parse_script(script: &[u8], data: &[u8]) -> Result<ScriptType> {
    if let Some(contract_calls) = parse_contract_calls(script, data)? {
        return Ok(ScriptType::ContractCall(contract_calls));
    }

    if let Some((script, blob_id)) = parse_loader_script(script, data) {
        return Ok(ScriptType::Loader(script, blob_id));
    }

    if let Some(script) = parse_script_call(script, data) {
        return Ok(ScriptType::Other(script));
    }

    unimplemented!()
}

fn parse_loader_script(script: &[u8], data: &[u8]) -> Option<(ScriptDescription, [u8; 32])> {
    // TODO: handle no data section
    let expected_loader_instructions = loader_instructions();

    // replace with split_at_checked when we move to msrv 1.80.0
    if script.len() < expected_loader_instructions.len() * Instruction::SIZE {
        return None;
    }

    let (instructions_part, remaining) =
        script.split_at(expected_loader_instructions.len() * Instruction::SIZE);

    if instructions_part.len() < expected_loader_instructions.len() * Instruction::SIZE {
        return None;
    }

    let instructions: Vec<Instruction> = fuel_asm::from_bytes(instructions_part.to_vec())
        .try_collect()
        .ok()?;

    if instructions
        .iter()
        .zip(expected_loader_instructions.iter())
        .any(|(actual, expected)| actual != expected)
    {
        return None;
    }

    // Should have enough for the blob id
    if remaining.len() < 32 {
        return None;
    }

    let blob_id = remaining[..32].try_into().unwrap();
    let remaining = &remaining[32..];

    // Should have enough for the data section len
    if remaining.len() < WORD_SIZE {
        return None;
    }
    let remaining = &remaining[WORD_SIZE..];

    Some((
        ScriptDescription {
            code: script.to_vec(),
            data: data.to_vec(),
            data_section_offset: Some(script.len().saturating_sub(remaining.len()) as u64),
        },
        blob_id,
    ))
}

#[cfg(test)]
mod tests {
    use fuel_asm::RegId;
    use fuels_core::types::errors::Error;
    use rand::{RngCore, SeedableRng};

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

    fn example_contract_call_data(gas_fwd: bool) -> Vec<u8> {
        // let amount = u64::from_be_bytes(script_data[..8].try_into().unwrap());
        // let data = &script_data[8..];
        //
        // let asset_id = AssetId::new(data[..32].try_into().unwrap());
        // let data = &data[32..];
        //
        // let contract_id = ContractId::new(data[..32].try_into().unwrap());
        // let data = &data[32..];
        //
        // let _fn_selector_offset = &data[..8];
        // let data = &data[8..];
        //
        // let _encoded_args_offset = &data[..8];
        // let data = &data[8..];
        //
        // let fn_selector_len = u64::from_be_bytes(data[..8].try_into().unwrap()) as usize;
        // let data = &data[8..];
        //
        // let fn_selector = String::from_utf8(data[..fn_selector_len].to_vec()).unwrap();
        // let data = &data[fn_selector_len..];
        //
        // let encoded_args = if self.has_gas_forwarding_instructions() {
        //     data[..data.len() - WORD_SIZE].to_vec()
        // } else {
        //     data.to_vec()
        // };
        // let data = &data[encoded_args.len()..];
        //
        // let gas_forwarded = self
        //     .has_gas_forwarding_instructions()
        //     .then(|| u64::from_be_bytes(data[..WORD_SIZE].try_into().unwrap()));
        //
        let mut data = vec![];
        data.extend_from_slice(&100u64.to_be_bytes());
        data.extend_from_slice(&[0; 32]);
        data.extend_from_slice(&[1; 32]);
        data.extend_from_slice(&[0; 8]);
        data.extend_from_slice(&[0; 8]);
        data.extend_from_slice(&"test".len().to_be_bytes());
        data.extend_from_slice("test".as_bytes());
        data.extend_from_slice(&[0; 8]);
        if gas_fwd {
            data.extend_from_slice(&[0; 8]);
        }
        data
    }

    #[test]
    fn contract_call_but_not_enough_data() {
        // given
        let script = get_single_call_instructions(&CallOpcodeParamsOffset {
            call_data_offset: 0,
            amount_offset: 0,
            asset_id_offset: 0,
            gas_forwarded_offset: None,
        })
        .unwrap();

        let ok_data = example_contract_call_data(false);
        let not_enough_data = ok_data[..ok_data.len() - 32].to_vec();

        // when
        let err = parse_script(&script, &not_enough_data).unwrap_err();

        // then
        let Error::Other(msg) = err else {
            panic!("expected Error::Other");
        };

        assert_eq!(msg, "not enough data for contract call");
    }
}
