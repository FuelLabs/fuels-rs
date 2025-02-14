use fuel_asm::{op, Instruction, RegId, Word};
use fuel_tx::{AssetId, ContractId};
use fuels_core::{constants::WORD_SIZE, error, types::errors::Result};

use super::cursor::WasmFriendlyCursor;
#[derive(Debug)]

pub struct ContractCallInstructions {
    instructions: Vec<Instruction>,
    gas_fwd: bool,
}

impl IntoIterator for ContractCallInstructions {
    type Item = Instruction;
    type IntoIter = std::vec::IntoIter<Instruction>;
    fn into_iter(self) -> Self::IntoIter {
        self.instructions.into_iter()
    }
}

impl ContractCallInstructions {
    pub fn new(opcode_params: CallOpcodeParamsOffset) -> Self {
        Self {
            gas_fwd: opcode_params.gas_forwarded_offset.is_some(),
            instructions: Self::generate_instructions(opcode_params),
        }
    }

    pub fn into_bytes(self) -> impl Iterator<Item = u8> {
        self.instructions
            .into_iter()
            .flat_map(|instruction| instruction.to_bytes())
    }

    /// Returns the VM instructions for calling a contract method
    /// We use the [`Opcode`] to call a contract: [`CALL`](Opcode::CALL)
    /// pointing at the following registers:
    ///
    /// 0x10 Script data offset
    /// 0x11 Coin amount
    /// 0x12 Asset ID
    /// 0x13 Gas forwarded
    ///
    /// Note that these are soft rules as we're picking this addresses simply because they
    /// non-reserved register.
    fn generate_instructions(offsets: CallOpcodeParamsOffset) -> Vec<Instruction> {
        let call_data_offset = offsets
            .call_data_offset
            .try_into()
            .expect("call_data_offset out of range");
        let amount_offset = offsets
            .amount_offset
            .try_into()
            .expect("amount_offset out of range");
        let asset_id_offset = offsets
            .asset_id_offset
            .try_into()
            .expect("asset_id_offset out of range");

        let mut instructions = [
            op::movi(0x10, call_data_offset),
            op::movi(0x11, amount_offset),
            op::lw(0x11, 0x11, 0),
            op::movi(0x12, asset_id_offset),
        ]
        .to_vec();

        match offsets.gas_forwarded_offset {
            Some(gas_forwarded_offset) => {
                let gas_forwarded_offset = gas_forwarded_offset
                    .try_into()
                    .expect("gas_forwarded_offset out of range");

                instructions.extend(&[
                    op::movi(0x13, gas_forwarded_offset),
                    op::lw(0x13, 0x13, 0),
                    op::call(0x10, 0x11, 0x12, 0x13),
                ]);
            }
            // if `gas_forwarded` was not set use `REG_CGAS`
            None => instructions.push(op::call(0x10, 0x11, 0x12, RegId::CGAS)),
        };

        instructions
    }

    fn extract_normal_variant(instructions: &[Instruction]) -> Option<&[Instruction]> {
        let normal_instructions = Self::generate_instructions(CallOpcodeParamsOffset {
            call_data_offset: 0,
            amount_offset: 0,
            asset_id_offset: 0,
            gas_forwarded_offset: None,
        });
        Self::extract_if_match(instructions, &normal_instructions)
    }

    fn extract_gas_fwd_variant(instructions: &[Instruction]) -> Option<&[Instruction]> {
        let gas_fwd_instructions = Self::generate_instructions(CallOpcodeParamsOffset {
            call_data_offset: 0,
            amount_offset: 0,
            asset_id_offset: 0,
            gas_forwarded_offset: Some(0),
        });
        Self::extract_if_match(instructions, &gas_fwd_instructions)
    }

    pub fn extract_from(instructions: &[Instruction]) -> Option<Self> {
        if let Some(instructions) = Self::extract_normal_variant(instructions) {
            return Some(Self {
                instructions: instructions.to_vec(),
                gas_fwd: false,
            });
        }

        Self::extract_gas_fwd_variant(instructions).map(|instructions| Self {
            instructions: instructions.to_vec(),
            gas_fwd: true,
        })
    }

    pub fn len(&self) -> usize {
        self.instructions.len()
    }

    pub fn call_data_offset(&self) -> u32 {
        let Instruction::MOVI(movi) = self.instructions[0] else {
            panic!("should have validated the first instruction is a MOVI");
        };

        movi.imm18().into()
    }

    pub fn is_gas_fwd_variant(&self) -> bool {
        self.gas_fwd
    }

    fn extract_if_match<'a>(
        unknown: &'a [Instruction],
        correct: &[Instruction],
    ) -> Option<&'a [Instruction]> {
        if unknown.len() < correct.len() {
            return None;
        }

        unknown
            .iter()
            .zip(correct)
            .all(|(expected, actual)| expected.opcode() == actual.opcode())
            .then(|| &unknown[..correct.len()])
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractCallData {
    pub amount: u64,
    pub asset_id: AssetId,
    pub contract_id: ContractId,
    pub fn_selector_encoded: Vec<u8>,
    pub encoded_args: Vec<u8>,
    pub gas_forwarded: Option<u64>,
}

impl ContractCallData {
    pub fn decode_fn_selector(&self) -> Result<String> {
        String::from_utf8(self.fn_selector_encoded.clone())
            .map_err(|e| error!(Codec, "cannot decode function selector: {}", e))
    }

    /// Encodes as script data, consisting of the following items in the given order:
    /// 1. Amount to be forwarded `(1 * `[`WORD_SIZE`]`)`
    /// 2. Asset ID to be forwarded ([`AssetId::LEN`])
    /// 3. Contract ID ([`ContractId::LEN`]);
    /// 4. Function selector offset `(1 * `[`WORD_SIZE`]`)`
    /// 5. Calldata offset `(1 * `[`WORD_SIZE`]`)`
    /// 6. Encoded function selector - method name
    /// 7. Encoded arguments
    /// 8. Gas to be forwarded `(1 * `[`WORD_SIZE`]`)` - Optional
    pub fn encode(&self, memory_offset: usize, buffer: &mut Vec<u8>) -> CallOpcodeParamsOffset {
        let amount_offset = memory_offset;
        let asset_id_offset = amount_offset + WORD_SIZE;
        let call_data_offset = asset_id_offset + AssetId::LEN;
        let encoded_selector_offset = call_data_offset + ContractId::LEN + 2 * WORD_SIZE;
        let encoded_args_offset = encoded_selector_offset + self.fn_selector_encoded.len();

        buffer.extend(self.amount.to_be_bytes()); // 1. Amount

        let asset_id = self.asset_id;
        buffer.extend(asset_id.iter()); // 2. Asset ID

        buffer.extend(self.contract_id.as_ref()); // 3. Contract ID

        buffer.extend((encoded_selector_offset as Word).to_be_bytes()); // 4. Fun. selector offset

        buffer.extend((encoded_args_offset as Word).to_be_bytes()); // 5. Calldata offset

        buffer.extend(&self.fn_selector_encoded); // 6. Encoded function selector

        let encoded_args_len = self.encoded_args.len();

        buffer.extend(&self.encoded_args); // 7. Encoded arguments

        let gas_forwarded_offset = self.gas_forwarded.map(|gf| {
            buffer.extend((gf as Word).to_be_bytes()); // 8. Gas to be forwarded - Optional

            encoded_args_offset + encoded_args_len
        });

        CallOpcodeParamsOffset {
            amount_offset,
            asset_id_offset,
            gas_forwarded_offset,
            call_data_offset,
        }
    }

    pub fn decode(data: &[u8], gas_fwd: bool) -> Result<Self> {
        let mut data = WasmFriendlyCursor::new(data);

        let amount = u64::from_be_bytes(data.consume_fixed("amount")?);

        let asset_id = AssetId::new(data.consume_fixed("asset id")?);

        let contract_id = ContractId::new(data.consume_fixed("contract id")?);

        let _ = data.consume(8, "function selector offset")?;

        let _ = data.consume(8, "encoded args offset")?;

        let fn_selector = {
            let fn_selector_len = {
                let bytes = data.consume_fixed("function selector length")?;
                u64::from_be_bytes(bytes) as usize
            };
            data.consume(fn_selector_len, "function selector")?.to_vec()
        };

        let (encoded_args, gas_forwarded) = if gas_fwd {
            let encoded_args = data
                .consume(data.unconsumed().saturating_sub(WORD_SIZE), "encoded_args")?
                .to_vec();

            let gas_fwd = { u64::from_be_bytes(data.consume_fixed("forwarded gas")?) };

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
}

#[derive(Default)]
/// Specifies offsets of [`Opcode::CALL`][`fuel_asm::Opcode::CALL`] parameters stored in the script
/// data from which they can be loaded into registers
pub struct CallOpcodeParamsOffset {
    pub call_data_offset: usize,
    pub amount_offset: usize,
    pub asset_id_offset: usize,
    pub gas_forwarded_offset: Option<usize>,
}

// Creates a contract that loads the specified blobs into memory and delegates the call to the code contained in the blobs.
pub fn loader_contract_asm(blob_ids: &[[u8; 32]]) -> Result<Vec<u8>> {
    const BLOB_ID_SIZE: u16 = 32;
    let get_instructions = |num_of_instructions, num_of_blobs| {
        // There are 2 main steps:
        // 1. Load the blob contents into memory
        // 2. Jump to the beginning of the memory where the blobs were loaded
        // After that the execution continues normally with the loaded contract reading our
        // prepared fn selector and jumps to the selected contract method.
        [
            // 1. Load the blob contents into memory
            // Find the start of the hardcoded blob IDs, which are located after the code ends.
            op::move_(0x10, RegId::PC),
            // 0x10 to hold the address of the current blob ID.
            op::addi(0x10, 0x10, num_of_instructions * Instruction::SIZE as u16),
            // The contract is going to be loaded from the current value of SP onwards, save
            // the location into 0x16 so we can jump into it later on.
            op::move_(0x16, RegId::SP),
            // Loop counter.
            op::movi(0x13, num_of_blobs),
            // LOOP starts here.
            // 0x11 to hold the size of the current blob.
            op::bsiz(0x11, 0x10),
            // Push the blob contents onto the stack.
            op::ldc(0x10, 0, 0x11, 1),
            // Move on to the next blob.
            op::addi(0x10, 0x10, BLOB_ID_SIZE),
            // Decrement the loop counter.
            op::subi(0x13, 0x13, 1),
            // Jump backwards (3+1) instructions if the counter has not reached 0.
            op::jnzb(0x13, RegId::ZERO, 3),
            // 2. Jump into the memory where the contract is loaded.
            // What follows is called _jmp_mem by the sway compiler.
            // Subtract the address contained in IS because jmp will add it back.
            op::sub(0x16, 0x16, RegId::IS),
            // jmp will multiply by 4, so we need to divide to cancel that out.
            op::divi(0x16, 0x16, 4),
            // Jump to the start of the contract we loaded.
            op::jmp(0x16),
        ]
    };

    let num_of_instructions = u16::try_from(get_instructions(0, 0).len())
        .expect("to never have more than u16::MAX instructions");

    let num_of_blobs = u32::try_from(blob_ids.len()).map_err(|_| {
        error!(
            Other,
            "the number of blobs ({}) exceeds the maximum number of blobs supported: {}",
            blob_ids.len(),
            u32::MAX
        )
    })?;

    let instruction_bytes = get_instructions(num_of_instructions, num_of_blobs)
        .into_iter()
        .flat_map(|instruction| instruction.to_bytes());

    let blob_bytes = blob_ids.iter().flatten().copied();

    Ok(instruction_bytes.chain(blob_bytes).collect())
}
