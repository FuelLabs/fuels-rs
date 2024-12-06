pub mod codec;
pub mod traits;
pub mod types;
mod utils;

use std::collections::HashSet;

use offsets::{extract_data_offset, extract_offset_at};
pub use utils::*;

use crate::types::errors::Result;

#[derive(Debug, Clone)]
pub enum Configurable {
    Direct {
        offset: u64,
        data: Vec<u8>,
    },
    Indirect {
        offset: u64,
        data_offset: u64,
        data: Vec<u8>,
    },
}

impl Configurable {
    pub fn data(&self) -> &[u8] {
        match self {
            Configurable::Direct { data, .. } => data,
            Configurable::Indirect { data, .. } => data,
        }
    }

    pub fn set_data(&mut self, new_data: Vec<u8>) {
        match self {
            Configurable::Direct { data, .. } => *data = new_data,
            Configurable::Indirect { data, .. } => *data = new_data,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Configurables {
    offsets_with_data: Vec<(u64, Vec<u8>)>,
    indirect_configurables: Vec<u64>,
}

impl Configurables {
    pub fn new(offsets_with_data: Vec<(u64, Vec<u8>)>, indirect_configurables: Vec<u64>) -> Self {
        Self {
            offsets_with_data,
            indirect_configurables,
        }
    }

    pub fn with_shifted_offsets(self, shift: i64) -> Result<Self> {
        let new_offsets_with_data = self
            .offsets_with_data
            .into_iter()
            .map(|(offset, data)| Ok((Self::shift_offset(offset, shift)?, data.clone())))
            .collect::<Result<Vec<_>>>()?;

        // TODO: @hal3e test this and thest with loader configurables
        let new_indirect_configurables = self
            .indirect_configurables
            .into_iter()
            .map(|offset| Self::shift_offset(offset, shift))
            .collect::<Result<Vec<_>>>()?;

        Ok(Self {
            offsets_with_data: new_offsets_with_data,
            indirect_configurables: new_indirect_configurables,
        })
    }

    fn shift_offset(offset: u64, shift: i64) -> Result<u64> {
        if shift.is_negative() {
            offset.checked_sub(shift.unsigned_abs())
        } else {
            offset.checked_add(shift.unsigned_abs())
        }
        .ok_or_else(|| {
            crate::error!(
                Other,
                "Overflow occurred while shifting offset: {offset} + {shift}"
            )
        })
    }

    pub fn update_constants_in(&self, binary: &mut [u8]) -> Result<()> {
        let data_offset = extract_data_offset(binary)?;
        let indirect_configurables: HashSet<u64> =
            HashSet::from_iter(self.indirect_configurables.iter().cloned());
        dbg!(&data_offset);

        for (offset, data) in &self.offsets_with_data {
            let offset = if indirect_configurables.contains(offset) {
                dbg!("indirect");
                extract_offset_at(binary, *offset as usize)? + data_offset
            } else {
                *offset as usize
            };
            dbg!(&offset);
            binary[offset..offset + data.len()].copy_from_slice(data)
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_with_shifted_offsets_positive_shift() {
        let offsets_with_data = vec![(10u64, vec![1, 2, 3])];
        let configurables = Configurables::new(offsets_with_data.clone(), vec![]);
        let shifted_configurables = configurables.with_shifted_offsets(5).unwrap();
        let expected_offsets_with_data = vec![(15u64, vec![1, 2, 3])];

        assert_eq!(
            shifted_configurables.offsets_with_data,
            expected_offsets_with_data
        );
    }

    #[test]
    fn test_with_shifted_offsets_negative_shift() {
        let offsets_with_data = vec![(10u64, vec![4, 5, 6])];
        let configurables = Configurables::new(offsets_with_data.clone(), vec![]);
        let shifted_configurables = configurables.with_shifted_offsets(-5).unwrap();
        let expected_offsets_with_data = vec![(5u64, vec![4, 5, 6])];

        assert_eq!(
            shifted_configurables.offsets_with_data,
            expected_offsets_with_data
        );
    }

    #[test]
    fn test_with_shifted_offsets_zero_shift() {
        let offsets_with_data = vec![(20u64, vec![7, 8, 9])];
        let configurables = Configurables::new(offsets_with_data.clone(), vec![]);
        let shifted_configurables = configurables.with_shifted_offsets(0).unwrap();
        let expected_offsets_with_data = offsets_with_data;

        assert_eq!(
            shifted_configurables.offsets_with_data,
            expected_offsets_with_data
        );
    }

    #[test]
    fn test_with_shifted_offsets_overflow() {
        let offsets_with_data = vec![(u64::MAX - 1, vec![1, 2, 3])];
        let configurables = Configurables::new(offsets_with_data, vec![]);
        let result = configurables.with_shifted_offsets(10);

        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e
                .to_string()
                .contains("Overflow occurred while shifting offset"));
        }
    }

    #[test]
    fn test_with_shifted_offsets_underflow() {
        let offsets_with_data = vec![(5, vec![4, 5, 6])];
        let configurables = Configurables::new(offsets_with_data, vec![]);
        let result = configurables.with_shifted_offsets(-10);

        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e
                .to_string()
                .contains("Overflow occurred while shifting offset"));
        }
    }
}
