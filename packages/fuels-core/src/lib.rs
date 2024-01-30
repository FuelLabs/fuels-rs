pub mod codec;
pub mod traits;
pub mod types;
mod utils;

use crate::codec::EncoderConfig;
pub use utils::*;

#[derive(Debug, Clone, Default)]
pub struct Configurables {
    offsets_with_data: Vec<(u64, Vec<u8>)>,
    // this is actually used by the `From<ConfigurableStruct>` implementation in `configurables.rs`
    #[allow(dead_code)]
    encoder_config: EncoderConfig,
}

impl Configurables {
    pub fn new(offsets_with_data: Vec<(u64, Vec<u8>)>, encoder_config: EncoderConfig) -> Self {
        Self {
            offsets_with_data,
            encoder_config,
        }
    }

    pub fn update_constants_in(&self, binary: &mut [u8]) {
        for (offset, data) in &self.offsets_with_data {
            let offset = *offset as usize;
            binary[offset..offset + data.len()].copy_from_slice(data)
        }
    }
}
