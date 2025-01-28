use std::{collections::HashMap, io::Read};

use fuel_abi_types::abi::unified_program::UnifiedProgramABI;
use itertools::Itertools;

use crate::{error, offsets::extract_offset_at, types::param_types::ParamType, Result};

use super::{ABIDecoder, DecoderConfig};

struct FormatterConfigurable {
    name: String,
    param_type: ParamType,
    offset: u64,
    indirect: bool,
}

pub struct ABIFormatter {
    functions: HashMap<String, Vec<ParamType>>,
    configurables: Vec<FormatterConfigurable>,
    decoder: ABIDecoder,
}

impl ABIFormatter {
    pub fn has_fn(&self, fn_name: &str) -> bool {
        self.functions.contains_key(fn_name)
    }

    pub fn with_decoder_config(mut self, config: DecoderConfig) -> Self {
        self.decoder = ABIDecoder::new(config);
        self
    }

    pub fn from_abi(abi: UnifiedProgramABI) -> Result<Self> {
        let functions = abi
            .functions
            .iter()
            .map(|fun| (fun.name.clone(), fun.clone()))
            .collect::<HashMap<_, _>>();

        let type_lookup = abi
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
                    .collect::<Result<Vec<_>>>()?;
                Ok((name.clone(), args))
            })
            .collect::<Result<HashMap<_, _>>>()?;

        let configurables = abi
            .configurables
            .into_iter()
            .flatten()
            .sorted_by_key(|c| c.offset)
            .map(|c| {
                let param_type =
                    ParamType::try_from_type_application(&c.application, &type_lookup)?;

                Ok(FormatterConfigurable {
                    name: c.name,
                    param_type,
                    offset: c.offset,
                    indirect: c.indirect,
                })
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(Self {
            functions,
            decoder: ABIDecoder::default(),
            configurables,
        })
    }

    pub fn from_json_abi(abi: impl AsRef<str>) -> Result<Self> {
        let parsed_abi = UnifiedProgramABI::from_json_abi(abi.as_ref())?;
        Self::from_abi(parsed_abi)
    }

    pub fn decode_fn_args<R: Read>(&self, fn_name: &str, data: R) -> Result<Vec<String>> {
        let args = self
            .functions
            .get(fn_name)
            .ok_or_else(|| error!(Codec, "Function '{}' not found in the ABI", fn_name))?;

        self.decoder.decode_multiple_as_debug_str(args, data)
    }

    pub fn decode_configurables(&self, configurable_data: &[u8]) -> Result<Vec<(String, String)>> {
        let min_offset = self
            .configurables
            .iter()
            .map(|c| c.offset)
            .min()
            .unwrap_or_default() as usize;

        self.configurables
            .iter()
            .map(|c| {
                let offset = (c.offset as usize).saturating_sub(min_offset);

                let decoded_string = if c.indirect {
                    let dyn_offset = extract_offset_at(configurable_data, offset)?;
                    self.decoder
                        .decode_as_debug_str(&c.param_type, &configurable_data[dyn_offset..])?
                } else {
                    self.decoder
                        .decode_as_debug_str(&c.param_type, &configurable_data[offset..])?
                };

                Ok((c.name.clone(), decoded_string))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::types::errors::Error;

    use super::*;

    #[test]
    fn gracefully_handles_missing_fn() {
        // given
        let decoder = ABIFormatter::from_abi(UnifiedProgramABI::default()).unwrap();

        // when
        let err = decoder
            .decode_fn_args("non_existent_fn", [].as_slice())
            .unwrap_err();

        // then
        let Error::Codec(err) = err else {
            panic!("Expected Codec error, got {:?}", err);
        };

        assert_eq!(err, "Function 'non_existent_fn' not found in the ABI");
    }
}
