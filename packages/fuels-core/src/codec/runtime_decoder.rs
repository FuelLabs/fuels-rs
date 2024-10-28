use std::collections::HashMap;

use fuel_abi_types::abi::unified_program::UnifiedProgramABI;

use crate::{types::param_types::ParamType, Result};

use super::{ABIDecoder, DecoderConfig};

pub struct ScriptArgsDecoder {
    decoder: RuntimeDecoder,
}
struct AbiConfigurable {
    name: String,
    param_type: ParamType,
    offset: u64,
}

pub struct RuntimeDecoder {
    functions: HashMap<String, Function>,
    configurables: Vec<AbiConfigurable>,
    decoder: ABIDecoder,
}

struct Function {
    name: String,
    args: Vec<ParamType>,
}

impl RuntimeDecoder {
    pub fn has_fn(&self, fn_name: &str) -> bool {
        self.functions.contains_key(fn_name)
    }

    pub fn with_decoder_config(mut self, config: DecoderConfig) -> Self {
        self.decoder = ABIDecoder::new(config);
        self
    }

    pub fn from_json_abi(abi: impl AsRef<str>) -> Result<Self> {
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

        let mut configurables: Vec<_> = parsed_abi
            .configurables
            .into_iter()
            .flatten()
            .map(|c| {
                let param_type =
                    ParamType::try_from_type_application(&c.application, &type_lookup).unwrap();

                AbiConfigurable {
                    name: c.name,
                    param_type,
                    offset: c.offset,
                }
            })
            .collect();

        let min_offset = configurables.iter().map(|c| c.offset).min().unwrap_or(0);

        configurables
            .iter_mut()
            .for_each(|c| c.offset -= min_offset);

        configurables.sort_by_key(|c| c.offset);

        Ok(Self {
            functions,
            decoder: ABIDecoder::default(),
            configurables,
        })
    }

    pub fn decode_fn_args(&self, fn_name: &str, data: &[u8]) -> Result<Vec<String>> {
        let fun = self.functions.get(fn_name).unwrap();
        self.decoder.decode_multiple_as_debug_str(&fun.args, data)
    }

    pub fn decode_configurables(&self, configurable_data: &[u8]) -> Result<Vec<(String, String)>> {
        eprintln!("configurable_data: {:?}", configurable_data);
        let param_types = self
            .configurables
            .iter()
            .map(|c| c.param_type.clone())
            .collect::<Vec<_>>();
        self.decoder
            .decode_multiple_as_debug_str(&param_types, configurable_data)?
            .into_iter()
            .zip(self.configurables.iter())
            .map(|(value, c)| Ok((c.name.clone(), value)))
            .collect()
    }
}
