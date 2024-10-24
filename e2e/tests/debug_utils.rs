use std::{any::Any, collections::HashMap, str::FromStr};

use fuel_abi_types::abi::{full_program::FullProgramABI, unified_program::UnifiedProgramABI};
use fuel_asm::{
    op::{self, MOVI},
    Instruction, Opcode,
};
use fuel_tx::field::ScriptData;
use fuels::{
    core::{codec::ABIDecoder, traits::Tokenizable},
    prelude::*,
    types::{param_types::ParamType, Bits256, EvmAddress, Identity, SizedAsciiString, B512, U256},
};
use logic::{parse_script, ContractCallArgsDecoder, ScriptArgsDecoder, ScriptType};

mod logic {
    use std::{borrow::Cow, collections::HashMap};

    use fuel_abi_types::abi::unified_program::{UnifiedABIFunction, UnifiedProgramABI};
    use fuel_asm::{Instruction, InvalidOpcode, Opcode};
    use fuel_tx::{AssetId, ContractId};
    use fuels::{
        core::{
            codec::{ABIDecoder, DecoderConfig},
            constants::WORD_SIZE,
        },
        prelude::Result,
        types::{errors::error, param_types::ParamType, transaction_builders::BlobId},
    };
    use itertools::Itertools;

    pub struct ContractCallDescription {
        pub amount: u64,
        pub asset_id: AssetId,
        pub contract_id: ContractId,
        pub fn_selector: String,
        pub encoded_args: Vec<u8>,
        pub gas_forwarded: Option<u64>,
    }

    pub struct ScriptDescription {
        // pub code: Vec<u8>,
        pub data: Vec<u8>,
        // pub data_section: Option<Vec<u8>>,
    }

    struct FnArgsDecoder {
        functions: HashMap<String, Function>,
        decoder: ABIDecoder,
    }

    pub struct ScriptArgsDecoder {
        decoder: FnArgsDecoder,
    }

    impl ScriptArgsDecoder {
        pub fn from_json_abi(abi: impl AsRef<str>) -> Result<Self> {
            // TODO: check that main fn exists
            let decoder = FnArgsDecoder::from_json_abi(abi)?;

            Ok(Self { decoder })
        }

        pub fn with_decoder_config(mut self, config: DecoderConfig) -> Self {
            self.decoder = self.decoder.with_decoder_config(config);
            self
        }

        pub fn decode_args(&self, script_description: &ScriptDescription) -> Result<Vec<String>> {
            self.decoder.decode("main", &script_description.data)
        }

        pub fn decode_configurables(
            &self,
            script_description: &ScriptDescription,
        ) -> Result<Vec<String>> {
            todo!()
        }
    }

    pub struct ContractCallArgsDecoder {
        decoder: FnArgsDecoder,
    }

    impl ContractCallArgsDecoder {
        pub fn from_json_abi(abi: impl AsRef<str>) -> Result<Self> {
            let decoder = FnArgsDecoder::from_json_abi(abi)?;
            Ok(Self { decoder })
        }

        pub fn with_decoder_config(mut self, config: DecoderConfig) -> Self {
            self.decoder = self.decoder.with_decoder_config(config);
            self
        }

        pub fn decode(&self, contract_call: &ContractCallDescription) -> Result<Vec<String>> {
            self.decoder
                .decode(&contract_call.fn_selector, &contract_call.encoded_args)
        }
    }

    struct Function {
        name: String,
        args: Vec<ParamType>,
    }

    impl FnArgsDecoder {
        fn has_fn(&self, fn_name: &str) -> bool {
            self.functions.contains_key(fn_name)
        }

        fn with_decoder_config(mut self, config: DecoderConfig) -> Self {
            self.decoder = ABIDecoder::new(config);
            self
        }

        fn from_json_abi(abi: impl AsRef<str>) -> Result<Self> {
            let parsed_abi = UnifiedProgramABI::from_json_abi(abi.as_ref())?;
            let functions = parsed_abi
                .functions
                .iter()
                .map(|fun| (fun.name.clone(), fun.clone()))
                .collect::<HashMap<_, _>>();

            let type_lookup = parsed_abi
                .types
                .iter()
                .map(|decl| (decl.type_id, decl.clone()))
                .collect::<HashMap<_, _>>();

            let functions = functions
                .into_iter()
                .map(|(name, fun)| {
                    let args = fun
                        .inputs
                        .iter()
                        .map(|type_application| {
                            ParamType::try_from_type_application(type_application, &type_lookup)
                        })
                        .collect::<Result<Vec<_>>>()
                        .unwrap();
                    (name.clone(), Function { name, args })
                })
                .collect::<HashMap<_, _>>();

            Ok(Self {
                functions,
                decoder: ABIDecoder::default(),
            })
        }
    }

    impl FnArgsDecoder {
        fn decode(&self, fn_name: &str, data: &[u8]) -> Result<Vec<String>> {
            let fun = self.functions.get(fn_name).unwrap();
            self.decoder.decode_multiple_as_debug_str(&fun.args, data)
        }
    }

    pub enum ScriptType {
        ContractCall(Vec<ContractCallDescription>),
        // LoaderScript(BlobId),
        Script(ScriptDescription),
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

        fn describe_contract_call(&self, script_data: &[u8]) -> ContractCallDescription {
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

            ContractCallDescription {
                amount,
                asset_id,
                contract_id,
                fn_selector,
                encoded_args,
                gas_forwarded,
            }
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

    fn parse_script_call(
        instructions: &[std::result::Result<Instruction, InvalidOpcode>],
        script_data: &[u8],
    ) -> Option<ScriptDescription> {
        // let instructions = instructions
        //     .into_iter()
        //     .enumerate()
        //     .filter_map(|(idx, instruction)| {
        //         match instruction {
        //             Ok(i) => Some(Ok(*i)),
        //             Err(_) if idx == 1 || idx == 2 => {
        //                 // we tolerate the 2nd and 3rd instructions being invalid since that is where
        //                 // sway encodes the data section offset
        //                 None
        //             }
        //             Err(e) => Some(Err(error!(Codec, "invalid instruction"))),
        //         }
        //     })
        //     .collect::<Result<Vec<Instruction>>>()
        //     .unwrap();

        return Some(ScriptDescription {
            data: script_data.to_vec(),
        });
    }

    fn parse_contract_calls(
        instructions: &[std::result::Result<Instruction, InvalidOpcode>],
        script_data: &[u8],
    ) -> Option<Vec<ContractCallDescription>> {
        let mut valid_instructions = vec![];

        for instruction in instructions {
            if let Ok(i) = instruction {
                valid_instructions.push(i.clone());
            } else {
                return None;
            }
        }

        let mut instructions = valid_instructions.as_slice();

        let mut call_instructions = vec![];

        while !instructions.is_empty() {
            match &instructions {
                [single_instruction] if single_instruction.opcode() == Opcode::RET => break,
                _ => {}
            }

            let (parsed_instructions, amount_read) =
                ContractCallInstructions::new(instructions).unwrap();
            instructions = &instructions[amount_read..];
            call_instructions.push(parsed_instructions);
        }

        let minimum_call_offset = call_instructions
            .iter()
            .map(|i| i.call_data_offset())
            .min()?;

        let mut descriptions = vec![];
        let num_calls = call_instructions.len();

        for (idx, current_call_instructions) in call_instructions.iter().enumerate() {
            let data_start =
                (current_call_instructions.call_data_offset() - minimum_call_offset) as usize;
            eprintln!("the offset is {data_start}");

            let data_end = if idx + 1 < num_calls {
                (call_instructions[idx + 1].call_data_offset()
                    - current_call_instructions.call_data_offset()) as usize
            } else {
                script_data.len()
            };

            eprintln!("the end is {data_end}");

            let contract_call_description = current_call_instructions
                .describe_contract_call(&script_data[data_start..data_end]);
            descriptions.push(contract_call_description);
        }

        Some(descriptions)
    }

    pub fn parse_script(script: &[u8], data: &[u8]) -> Result<ScriptType> {
        let instructions = fuel_asm::from_bytes(script.to_vec()).collect_vec();

        if let Some(contract_calls) = parse_contract_calls(instructions.as_slice(), data) {
            return Ok(ScriptType::ContractCall(contract_calls));
        }

        if let Some(script) = parse_script_call(&instructions, data) {
            return Ok(ScriptType::Script(script));
        }

        unimplemented!()
    }
}

#[tokio::test]
async fn can_debug_single_call_tx() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "MyContract",
            project = "e2e/sway/types/contracts/nested_structs"
        ))
    );
    let contract_id = Contract::load_from(
        "sway/types/contracts/nested_structs/out/release/nested_structs.bin",
        Default::default(),
    )?
    .contract_id();

    let call_handler = MyContract::new(contract_id, wallet)
        .methods()
        .check_struct_integrity(AllStruct {
            some_struct: SomeStruct {
                field: 2,
                field_2: true,
            },
        });

    let abi = std::fs::read_to_string(
        "./sway/types/contracts/nested_structs/out/release/nested_structs-abi.json",
    )
    .unwrap();
    let decoder = ContractCallArgsDecoder::from_json_abi(&abi)?;

    // without gas forwarding
    {
        let tb = call_handler
            .clone()
            .call_params(CallParameters::default().with_amount(10))
            .unwrap()
            .transaction_builder()
            .await
            .unwrap();

        let script = tb.script;
        let script_data = tb.script_data;

        let ScriptType::ContractCall(call_descriptions) =
            logic::parse_script(&script, &script_data)?
        else {
            panic!("expected a contract call")
        };

        assert_eq!(call_descriptions.len(), 1);
        let call_description = &call_descriptions[0];

        assert_eq!(call_description.contract_id, contract_id);
        assert_eq!(call_description.amount, 10);
        assert_eq!(call_description.asset_id, AssetId::default());
        assert_eq!(call_description.fn_selector, "check_struct_integrity");
        assert!(call_description.gas_forwarded.is_none());

        assert_eq!(
            decoder.decode(call_description)?,
            vec!["AllStruct { some_struct: SomeStruct { field: 2, field_2: true } }"]
        );
    }

    // with gas forwarding
    {
        let tb = call_handler
            .clone()
            .call_params(
                CallParameters::default()
                    .with_amount(10)
                    .with_gas_forwarded(20),
            )
            .unwrap()
            .transaction_builder()
            .await
            .unwrap();

        let script = tb.script;
        let script_data = tb.script_data;

        let ScriptType::ContractCall(call_descriptions) =
            logic::parse_script(&script, &script_data)?
        else {
            panic!("expected a contract call")
        };

        assert_eq!(call_descriptions.len(), 1);
        let call_description = &call_descriptions[0];

        assert_eq!(call_description.contract_id, contract_id);
        assert_eq!(call_description.amount, 10);
        assert_eq!(call_description.asset_id, AssetId::default());
        assert_eq!(call_description.fn_selector, "check_struct_integrity");
        assert_eq!(call_description.gas_forwarded, Some(20));

        assert_eq!(
            decoder.decode(call_description)?,
            vec!["AllStruct { some_struct: SomeStruct { field: 2, field_2: true } }"]
        );
    }

    Ok(())
}

#[tokio::test]
async fn can_debug_multi_call_tx() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "MyContract",
            project = "e2e/sway/types/contracts/nested_structs"
        ))
    );
    let contract_id = Contract::load_from(
        "sway/types/contracts/nested_structs/out/release/nested_structs.bin",
        Default::default(),
    )?
    .contract_id();

    let call1 = MyContract::new(contract_id, wallet.clone())
        .methods()
        .check_struct_integrity(AllStruct {
            some_struct: SomeStruct {
                field: 2,
                field_2: true,
            },
        });

    let call2 = MyContract::new(contract_id, wallet.clone())
        .methods()
        .i_am_called_differently(
            AllStruct {
                some_struct: SomeStruct {
                    field: 2,
                    field_2: true,
                },
            },
            MemoryAddress {
                contract_id,
                function_selector: 123,
                function_data: 456,
            },
        );

    let abi = std::fs::read_to_string(
        "./sway/types/contracts/nested_structs/out/release/nested_structs-abi.json",
    )
    .unwrap();
    let decoder = ContractCallArgsDecoder::from_json_abi(&abi)?;

    // without gas forwarding
    {
        let first_call = call1
            .clone()
            .call_params(CallParameters::default().with_amount(10))
            .unwrap();

        let second_call = call2
            .clone()
            .call_params(CallParameters::default().with_amount(20))
            .unwrap();

        let tb = CallHandler::new_multi_call(wallet.clone())
            .add_call(first_call)
            .add_call(second_call)
            .transaction_builder()
            .await
            .unwrap();

        let script = tb.script;
        let script_data = tb.script_data;

        let ScriptType::ContractCall(call_descriptions) =
            logic::parse_script(&script, &script_data)?
        else {
            panic!("expected a contract call")
        };

        assert_eq!(call_descriptions.len(), 2);

        let call_description = &call_descriptions[0];

        assert_eq!(call_description.contract_id, contract_id);
        assert_eq!(call_description.amount, 10);
        assert_eq!(call_description.asset_id, AssetId::default());
        assert_eq!(call_description.fn_selector, "check_struct_integrity");
        assert!(call_description.gas_forwarded.is_none());

        assert_eq!(
            decoder.decode(call_description)?,
            vec!["AllStruct { some_struct: SomeStruct { field: 2, field_2: true } }"]
        );

        let call_description = &call_descriptions[1];

        assert_eq!(call_description.contract_id, contract_id);
        assert_eq!(call_description.amount, 20);
        assert_eq!(call_description.asset_id, AssetId::default());
        assert_eq!(call_description.fn_selector, "i_am_called_differently");
        assert!(call_description.gas_forwarded.is_none());

        assert_eq!(
            decoder.decode(call_description)?,
            vec!["AllStruct { some_struct: SomeStruct { field: 2, field_2: true } }", "MemoryAddress { contract_id: std::contract_id::ContractId { bits: Bits256([30, 98, 236, 170, 92, 50, 241, 229, 25, 84, 244, 97, 73, 213, 229, 66, 71, 43, 219, 164, 88, 56, 25, 148, 6, 70, 74, 244, 106, 177, 71, 237]) }, function_selector: 123, function_data: 456 }"]
        );
    }

    // with gas forwarding
    {
        let first_call = call1
            .clone()
            .call_params(
                CallParameters::default()
                    .with_amount(10)
                    .with_gas_forwarded(15),
            )
            .unwrap();

        let second_call = call2
            .clone()
            .call_params(
                CallParameters::default()
                    .with_amount(20)
                    .with_gas_forwarded(25),
            )
            .unwrap();

        let tb = CallHandler::new_multi_call(wallet.clone())
            .add_call(first_call)
            .add_call(second_call)
            .transaction_builder()
            .await
            .unwrap();

        let script = tb.script;
        let script_data = tb.script_data;

        let ScriptType::ContractCall(call_descriptions) =
            logic::parse_script(&script, &script_data)?
        else {
            panic!("expected a contract call")
        };

        assert_eq!(call_descriptions.len(), 2);

        let call_description = &call_descriptions[0];

        assert_eq!(call_description.contract_id, contract_id);
        assert_eq!(call_description.amount, 10);
        assert_eq!(call_description.asset_id, AssetId::default());
        assert_eq!(call_description.fn_selector, "check_struct_integrity");
        assert_eq!(call_description.gas_forwarded, Some(15));

        assert_eq!(
            decoder.decode(call_description)?,
            vec!["AllStruct { some_struct: SomeStruct { field: 2, field_2: true } }"]
        );

        let call_description = &call_descriptions[1];

        assert_eq!(call_description.contract_id, contract_id);
        assert_eq!(call_description.amount, 20);
        assert_eq!(call_description.asset_id, AssetId::default());
        assert_eq!(call_description.fn_selector, "i_am_called_differently");
        assert_eq!(call_description.gas_forwarded, Some(25));

        assert_eq!(
            decoder.decode(call_description)?,
            vec!["AllStruct { some_struct: SomeStruct { field: 2, field_2: true } }", "MemoryAddress { contract_id: std::contract_id::ContractId { bits: Bits256([30, 98, 236, 170, 92, 50, 241, 229, 25, 84, 244, 97, 73, 213, 229, 66, 71, 43, 219, 164, 88, 56, 25, 148, 6, 70, 74, 244, 106, 177, 71, 237]) }, function_selector: 123, function_data: 456 }"]
        );
    }

    Ok(())
}

#[tokio::test]
async fn can_debug_sway_script() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Script(
            name = "MyScript",
            project = "e2e/sway/scripts/script_struct"
        )),
        LoadScript(
            name = "script_instance",
            script = "MyScript",
            wallet = "wallet"
        )
    );

    let tb = script_instance
        .main(MyStruct {
            number: 10,
            boolean: false,
        })
        .transaction_builder()
        .await
        .unwrap();

    let abi =
        std::fs::read_to_string("./sway/scripts/script_struct/out/release/script_struct-abi.json")?;

    let decoder = ScriptArgsDecoder::from_json_abi(&abi)?;

    let ScriptType::Script(desc) = parse_script(&tb.script, &tb.script_data).unwrap() else {
        panic!("expected a script")
    };

    assert_eq!(
        decoder.decode_args(&desc)?,
        vec!["MyStruct { number: 10, boolean: false }"]
    );

    assert_eq!(
        decoder.decode_configurables(&desc).unwrap(),
        vec!["MyStruct { number: 10, boolean: false }", "11"]
    );

    Ok(())
}
