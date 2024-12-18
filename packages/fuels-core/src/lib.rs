pub mod codec;
pub mod traits;
pub mod types;
mod utils;

use std::{collections::HashMap, iter};

use codec::ABIEncoder;
use itertools::Itertools;
use offsets::{extract_data_offset, extract_offset_at};
use traits::Tokenizable;
pub use utils::*;

use crate::types::errors::Result;

type OffsetWithData = (u64, Vec<u8>);
type OffsetWithSlice<'a> = (u64, &'a [u8]);

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
    offsets_with_data: Vec<OffsetWithData>,
    sorted_indirect_configurables: Vec<u64>,
}

impl Configurables {
    pub fn new(offsets_with_data: Vec<OffsetWithData>, indirect_configurables: Vec<u64>) -> Self {
        let sorted_indirect_configurables = indirect_configurables
            .into_iter()
            .sorted_unstable()
            .collect();

        Self {
            offsets_with_data,
            sorted_indirect_configurables,
        }
    }

    pub fn with_shifted_offsets(self, shift: i64) -> Result<Self> {
        let new_offsets_with_data = self
            .offsets_with_data
            .into_iter()
            .map(|(offset, data)| Ok((Self::shift_offset(offset, shift)?, data.clone())))
            .collect::<Result<Vec<_>>>()?;

        // TODO: @hal3e test this and thest with loader configurables
        let new_sorted_indirect_configurables = self
            .sorted_indirect_configurables
            .into_iter()
            .map(|offset| Self::shift_offset(offset, shift))
            .collect::<Result<Vec<_>>>()?;

        Ok(Self {
            offsets_with_data: new_offsets_with_data,
            sorted_indirect_configurables: new_sorted_indirect_configurables,
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
        let (direct_configurables, indirect_configurables) =
            self.partition_direct_indirect_configurables();

        Self::apply_direct_configurables(binary, &direct_configurables)?;

        if !indirect_configurables.is_empty() {
            self.apply_indirect_configurables(binary, &indirect_configurables)?;
        }

        Ok(())
    }

    fn partition_direct_indirect_configurables(
        &self,
    ) -> (Vec<OffsetWithSlice>, Vec<OffsetWithSlice>) {
        self.offsets_with_data
            .iter()
            .map(|(offset, data)| (*offset, data.as_slice()))
            .partition(|(offset, _)| {
                self.sorted_indirect_configurables
                    .binary_search(offset)
                    .is_err()
            })
    }

    fn apply_direct_configurables(
        binary: &mut [u8],
        direct_configurables: &[OffsetWithSlice],
    ) -> Result<()> {
        for (offset, data) in direct_configurables {
            Self::write(binary, *offset as usize, data)?;
        }

        Ok(())
    }

    fn apply_indirect_configurables(
        &self,
        binary: &mut Vec<u8>,
        indirect_configurables: &[OffsetWithSlice],
    ) -> Result<()> {
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

        let sorted_dyn_offsets = self.extract_sorted_dyn_offsets(binary, data_offset)?;

        for offset in &self.sorted_indirect_configurables {
            if change_map.contains_key(offset) {
                continue;
            }

            let dyn_offset = extract_offset_at(binary, *offset as usize)? + data_offset;

            if dyn_offset < min_offset {
                continue;
            }

            // use the next dyn offset to know where the data ends
            let idx = sorted_dyn_offsets
                .binary_search(&dyn_offset)
                .expect("is there as we created the sorted vec");
            let end_offset = sorted_dyn_offsets[idx + 1]; // is there as we created the sorted vec

            Self::check_binary_len(binary, dyn_offset)?;
            Self::check_binary_len(binary, end_offset)?;

            let data = &binary[dyn_offset..end_offset];

            change_map.insert(*offset, (dyn_offset, data));
        }

        // cut old binary and append the updated dynamic data
        let mut new_binary = binary[..min_offset].to_vec();

        for offset in &self.sorted_indirect_configurables {
            if let Some((_, data)) = change_map.get(offset) {
                let new_offset = new_binary.len().saturating_sub(data_offset) as u64;
                let new_offset_encoded =
                    ABIEncoder::default().encode(&[new_offset.into_token()])?;

                Self::write(
                    new_binary.as_mut_slice(),
                    *offset as usize,
                    &new_offset_encoded,
                )?;

                new_binary.extend(*data);
            }
        }

        *binary = new_binary;

        Ok(())
    }

    fn extract_sorted_dyn_offsets(&self, binary: &[u8], data_offset: usize) -> Result<Vec<usize>> {
        Ok(self
            .sorted_indirect_configurables
            .iter()
            .cloned()
            .map(|offset| Ok(extract_offset_at(binary, offset as usize)? + data_offset))
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .chain(iter::once(binary.len()))
            .sorted_unstable()
            .collect())
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
