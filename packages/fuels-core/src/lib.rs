pub mod codec;
pub mod traits;
pub mod types;
mod utils;

pub use utils::*;

use crate::types::errors::Result;

#[derive(Debug, Clone, Default)]
pub struct Configurables {
    offsets_with_data: Vec<(u64, Vec<u8>)>,
}

impl Configurables {
    pub fn new(offsets_with_data: Vec<(u64, Vec<u8>)>) -> Self {
        Self { offsets_with_data }
    }

    pub fn update_constants_in(&self, binary: &mut [u8]) -> Result<()> {
        for (offset, data) in &self.offsets_with_data {
            let offset_start = *offset as usize;
            let offset_end = offset_start
                .checked_add(data.len())
                .ok_or_else(|| error!(InvalidType, "Addition overflow while calculating offset"))?;
            binary[offset_start..offset_end].copy_from_slice(data)
        }
        Ok(())
    }
}
