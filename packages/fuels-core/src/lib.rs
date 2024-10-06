pub mod codec;
pub mod traits;
pub mod types;
mod utils;

pub use utils::*;

use crate::types::errors::Result;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Configurables {
    offsets_with_data: Vec<(u64, Vec<u8>)>,
}

impl Configurables {
    pub fn new(offsets_with_data: Vec<(u64, Vec<u8>)>) -> Self {
        Self { offsets_with_data }
    }

    // TODO: test
    pub fn with_shifted_offsets(self, shift: i64) -> Result<Self> {
        self.offsets_with_data
            .iter()
            .map(|(offset, data)| {
                let offset = offset.checked_add(shift as u64).ok_or_else(|| {
                    crate::error!(
                        Other,
                        "Overflow occurred while shifting offset: {} + {}",
                        offset,
                        shift
                    )
                })?;
                Ok((offset, data.clone()))
            })
            .collect::<Result<Vec<_>>>()
            .map(Self::new)
    }

    pub fn update_constants_in(&self, binary: &mut [u8]) {
        for (offset, data) in &self.offsets_with_data {
            let offset = *offset as usize;
            binary[offset..offset + data.len()].copy_from_slice(data)
        }
    }
}
