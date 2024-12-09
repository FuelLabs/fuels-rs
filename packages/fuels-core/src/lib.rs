pub mod codec;
pub mod traits;
pub mod types;
mod utils;

use std::collections::{HashMap, HashSet};

use codec::ABIEncoder;
use itertools::{Either, Itertools};
use offsets::{extract_data_offset, extract_offset_at};
use traits::Tokenizable;
pub use utils::*;

use crate::types::errors::Result;

#[derive(Debug, Clone)]
pub enum Configurable {
    //TODO:hal3e make private
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

    pub fn update_constants_in(&self, binary: &mut Vec<u8>) -> Result<()> {
        let indirect_configurables_sorted = self
            .indirect_configurables
            .iter()
            .cloned()
            .sorted_unstable()
            .collect_vec();

        let indirect_configurables_map: HashSet<u64> =
            HashSet::from_iter(self.indirect_configurables.iter().cloned());

        let (direct_configurables, indirect_configurables): (Vec<_>, Vec<_>) = self
            .offsets_with_data
            .iter()
            .partition_map(|(offset, data)| {
                if indirect_configurables_map.contains(offset) {
                    Either::Right((*offset, data.as_slice()))
                } else {
                    Either::Left((*offset, data.as_slice()))
                }
            });

        Self::apply_direct_configurables(binary, &direct_configurables)?;

        if !indirect_configurables.is_empty() {
            let data_offset = extract_data_offset(binary)?;

            let mut change_map: HashMap<u64, (usize, &[u8])> = indirect_configurables
                .iter()
                .map(|(offset, data)| {
                    let dyn_offset = extract_offset_at(binary, *offset as usize)? + data_offset;

                    Ok((*offset, (dyn_offset, *data)))
                })
                .collect::<Result<HashMap<_, _>>>()?;

            let min_offset = change_map
                .values()
                .map(|(dyn_offset, _)| *dyn_offset)
                .min()
                .expect("exists");

            for (offset, next_offset) in indirect_configurables_sorted //TODO: @hal3e refactor
                .iter()
                .circular_tuple_windows()
            {
                if change_map.contains_key(offset) {
                    continue;
                }

                let dyn_offset = extract_offset_at(binary, *offset as usize)? + data_offset;

                if dyn_offset < min_offset {
                    continue;
                }

                let end_offset = if next_offset <= offset {
                    binary.len()
                } else {
                    extract_offset_at(binary, *next_offset as usize)? + data_offset
                };

                Self::check_binary_len(binary, dyn_offset)?;
                Self::check_binary_len(binary, end_offset)?;

                let data = &binary[dyn_offset..end_offset];

                change_map.insert(*offset, (dyn_offset, data));
            }

            let mut new_binary = binary[..min_offset].to_vec();
            // go through the configurables and apply the changes
            for offset in indirect_configurables_sorted {
                if let Some((_, data)) = change_map.get(&offset) {
                    let new_offset = new_binary.len().saturating_sub(data_offset) as u64;
                    let new_offset_encoded =
                        ABIEncoder::default().encode(&[new_offset.into_token()])?;
                    Self::write(
                        new_binary.as_mut_slice(),
                        offset as usize,
                        &new_offset_encoded,
                    )?;

                    new_binary.extend(*data);
                }
            }

            *binary = new_binary;
        }

        Ok(())
    }

    fn apply_direct_configurables(binary: &mut [u8], configurables: &[(u64, &[u8])]) -> Result<()> {
        for (offset, data) in configurables {
            Self::write(binary, *offset as usize, data)?;
        }

        Ok(())
    }

    fn write(binary: &mut [u8], offset: usize, data: &[u8]) -> Result<()> {
        let data_len = data.len();
        Self::check_binary_len(binary, offset + data_len)?;

        binary[offset..offset + data.len()].copy_from_slice(data);

        Ok(())
    }

    fn check_binary_len(binary: &[u8], offset: usize) -> Result<()> {
        if binary.len() < offset {
            return Err(crate::error!(
                Other,
                "configurables: given binary with len: `{}` is too short for offset:`{}`",
                binary.len(),
                offset
            ));
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
