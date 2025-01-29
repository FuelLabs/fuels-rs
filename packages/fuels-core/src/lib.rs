pub mod codec;
pub mod traits;
pub mod types;
mod utils;

use std::{collections::HashMap, iter, path::Path};

use codec::{try_from_bytes, ABIEncoder, DecoderConfig};
use itertools::Itertools;
use offsets::{extract_data_offset, extract_offset_at};
use traits::{Parameterize, Tokenizable};
pub use utils::*;

use crate::types::errors::Result;

type OffsetWithData = (u64, Vec<u8>);
type OffsetWithSlice<'a> = (u64, &'a [u8]);

#[derive(Debug, Clone)]
pub struct ConfigurablesReader {
    binary: Vec<u8>,
    decoder_config: DecoderConfig,
}

impl ConfigurablesReader {
    pub fn load(binary: Vec<u8>) -> Self {
        Self {
            binary,
            decoder_config: DecoderConfig::default(),
        }
    }

    pub fn load_from(binary_filepath: impl AsRef<Path>) -> Result<Self> {
        let binary_filepath = binary_filepath.as_ref();

        let binary = std::fs::read(binary_filepath).map_err(|e| {
            std::io::Error::new(
                e.kind(),
                format!("failed to read binary: {binary_filepath:?}: {e}"),
            )
        })?;

        Ok(Self {
            binary,
            decoder_config: DecoderConfig::default(),
        })
    }

    pub fn with_decoder_config(mut self, decoder_config: DecoderConfig) -> Self {
        self.decoder_config = decoder_config;

        self
    }

    pub fn decode_direct<T: Tokenizable + Parameterize>(&self, offset: usize) -> Result<T> {
        check_binary_len(&self.binary, offset)?;

        try_from_bytes(&self.binary[offset..], self.decoder_config)
    }

    pub fn decode_indirect<T: Tokenizable + Parameterize>(&self, offset: usize) -> Result<T> {
        let data_offset = extract_data_offset(&self.binary)?;
        let dyn_offset = extract_offset_at(&self.binary, offset)?;

        check_binary_len(&self.binary, data_offset + dyn_offset)?;

        try_from_bytes(
            &self.binary[data_offset + dyn_offset..],
            self.decoder_config,
        )
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Configurables {
    offsets_with_data: Vec<OffsetWithData>,
    sorted_indirect_offsets: Vec<u64>,
}

impl Configurables {
    pub fn new(offsets_with_data: Vec<OffsetWithData>, indirect_configurables: Vec<u64>) -> Self {
        let sorted_indirect_offsets = indirect_configurables
            .into_iter()
            .sorted_unstable()
            .collect();

        Self {
            offsets_with_data,
            sorted_indirect_offsets,
        }
    }

    pub fn with_shifted_offsets(self, shift: i64) -> Result<Self> {
        let new_offsets_with_data = self
            .offsets_with_data
            .into_iter()
            .map(|(offset, data)| Ok((Self::shift_offset(offset, shift)?, data.clone())))
            .collect::<Result<Vec<_>>>()?;

        let new_sorted_indirect_configurables = self
            .sorted_indirect_offsets
            .into_iter()
            .map(|offset| Self::shift_offset(offset, shift))
            .collect::<Result<Vec<_>>>()?;

        Ok(Self {
            offsets_with_data: new_offsets_with_data,
            sorted_indirect_offsets: new_sorted_indirect_configurables,
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
                "overflow occurred while shifting configurable's offset: {offset} + {shift}"
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
            .partition(|(offset, _)| self.sorted_indirect_offsets.binary_search(offset).is_err())
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

        for offset in &self.sorted_indirect_offsets {
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

            check_binary_len(binary, dyn_offset)?;
            check_binary_len(binary, end_offset)?;

            let data = &binary[dyn_offset..end_offset];

            change_map.insert(*offset, (dyn_offset, data));
        }

        // cut old binary and append the updated dynamic data
        let mut new_binary = binary[..min_offset].to_vec();

        for offset in &self.sorted_indirect_offsets {
            if let Some((_, data)) = change_map.get(offset) {
                let new_dyn_offset = new_binary.len().saturating_sub(data_offset) as u64;
                let new_dyn_offset_encoded =
                    ABIEncoder::default().encode(&[new_dyn_offset.into_token()])?;

                Self::write(
                    new_binary.as_mut_slice(),
                    *offset as usize,
                    &new_dyn_offset_encoded,
                )?;

                new_binary.extend(*data);
            }
        }

        *binary = new_binary;

        Ok(())
    }

    fn extract_sorted_dyn_offsets(&self, binary: &[u8], data_offset: usize) -> Result<Vec<usize>> {
        Ok(self
            .sorted_indirect_offsets
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
        check_binary_len(binary, offset + data_len)?;

        binary[offset..offset + data.len()].copy_from_slice(data);

        Ok(())
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_with_shifted_offsets_positive_shift() {
        let offsets_with_data = vec![(10, vec![1, 2, 3]), (20, vec![4, 5, 6])];
        let indirect_configurables = vec![20, 10];
        let configurables = Configurables::new(offsets_with_data.clone(), indirect_configurables);

        let shifted_configurables = configurables.with_shifted_offsets(5).unwrap();
        let expected_offsets_with_data = vec![(15, vec![1, 2, 3]), (25, vec![4, 5, 6])];
        let expected_sorted_indirect_configurables = vec![15, 25];

        assert_eq!(
            shifted_configurables.offsets_with_data,
            expected_offsets_with_data
        );
        assert_eq!(
            shifted_configurables.sorted_indirect_offsets,
            expected_sorted_indirect_configurables
        );
    }

    #[test]
    fn test_with_shifted_offsets_negative_shift() {
        let offsets_with_data = vec![(10, vec![4, 5, 6]), (30, vec![7, 8, 9])];
        let indirect_configurables = vec![30, 10];
        let configurables = Configurables::new(offsets_with_data.clone(), indirect_configurables);

        let shifted_configurables = configurables.with_shifted_offsets(-5).unwrap();
        let expected_offsets_with_data = vec![(5, vec![4, 5, 6]), (25, vec![7, 8, 9])];
        let expected_sorted_indirect_configurables = vec![5, 25];

        assert_eq!(
            shifted_configurables.offsets_with_data,
            expected_offsets_with_data
        );
        assert_eq!(
            shifted_configurables.sorted_indirect_offsets,
            expected_sorted_indirect_configurables
        );
    }

    #[test]
    fn test_with_shifted_offsets_zero_shift() {
        let offsets_with_data = vec![(20, vec![7, 8, 9]), (40, vec![10, 11, 12])];
        let indirect_configurables = vec![40, 20];
        let configurables = Configurables::new(offsets_with_data.clone(), indirect_configurables);

        let shifted_configurables = configurables.with_shifted_offsets(0).unwrap();
        let expected_offsets_with_data = offsets_with_data;
        let expected_sorted_indirect_configurables = vec![20, 40];

        assert_eq!(
            shifted_configurables.offsets_with_data,
            expected_offsets_with_data
        );
        assert_eq!(
            shifted_configurables.sorted_indirect_offsets,
            expected_sorted_indirect_configurables
        );
    }

    #[test]
    fn test_with_shifted_offsets_overflow() {
        let offsets_with_data = vec![(u64::MAX - 1, vec![1, 2, 3]), (u64::MAX - 2, vec![4, 5, 6])];
        let indirect_configurables = vec![u64::MAX - 1, u64::MAX - 2];
        let configurables = Configurables::new(offsets_with_data, indirect_configurables);

        let result = configurables.with_shifted_offsets(10);

        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("overflow occurred while shifting"));
        }
    }

    #[test]
    fn test_with_shifted_offsets_underflow() {
        let offsets_with_data = vec![(5, vec![4, 5, 6]), (15, vec![7, 8, 9])];
        let indirect_configurables = vec![15, 5];
        let configurables = Configurables::new(offsets_with_data, indirect_configurables);

        let result = configurables.with_shifted_offsets(-10);

        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("overflow occurred while shifting"));
        }
    }

    fn generate_test_binary() -> Vec<u8> {
        let mut binary = vec![0, 1, 2, 3, 4, 5, 6];

        let direct_offsets = [7, 9, 11];
        let direct_data = [[7, 8], [9, 10], [11, 12]];

        // Write direct configurables
        for data in direct_data {
            binary.extend(data);
        }

        let indirect_offsets = [13, 14, 15];
        let indirect_data = [vec![13, 14, 15], vec![16, 17, 18], vec![19, 20, 21]];

        let data_section_offset = 180; // Arbitrary offset for dynamic data

        // Write indirect configurable pointers
        for (i, &offset) in indirect_offsets.iter().enumerate() {
            let pointer = (data_section_offset + (i * 4)) as u64;
            let encoded_offset = ABIEncoder::default()
                .encode(&[pointer.into_token()])
                .unwrap();
            binary[offset..offset + encoded_offset.len()].copy_from_slice(&encoded_offset);
        }

        // Write dynamic data
        for (i, data) in indirect_data.iter().enumerate() {
            let data_position = data_section_offset + (i * 4);
            binary[data_position..data_position + data.len()].copy_from_slice(data);
        }

        binary
    }

    fn setup_configurables() -> Configurables {
        let offsets_with_data = vec![
            OffsetWithData {
                offset: 20,
                data: vec![0xAA, 0xBB],
            },
            OffsetWithData {
                offset: 50,
                data: vec![0xCC, 0xDD],
            },
            OffsetWithData {
                offset: 120,
                data: vec![0xEE, 0xFF],
            },
            OffsetWithData {
                offset: 30,
                data: vec![1, 2, 3, 4],
            },
            OffsetWithData {
                offset: 80,
                data: vec![5, 6, 7, 8],
            },
            OffsetWithData {
                offset: 160,
                data: vec![9, 10, 11, 12],
            },
        ];

        let indirect_configurables = vec![30, 80, 160]; // These are pointers

        Configurables::new(offsets_with_data, indirect_configurables)
    }

    #[test]
    fn test_update_first_indirect_configurable() {
        let mut binary = generate_test_binary();
        let mut configurables = setup_configurables();

        // Modify first indirect configurable data
        configurables.offsets_with_data[3].data = vec![99, 98, 97, 96];

        configurables.update_constants_in(&mut binary).unwrap();

        let new_offset = extract_offset_at(&binary, 30).unwrap();
        let new_data = &binary[new_offset as usize..(new_offset as usize) + 4];

        assert_eq!(new_data, &[99, 98, 97, 96]);
    }

    #[test]
    fn test_update_middle_indirect_configurable() {
        let mut binary = generate_test_binary();
        let mut configurables = setup_configurables();

        // Modify middle indirect configurable data
        configurables.offsets_with_data[4].data = vec![55, 66, 77, 88];

        configurables.update_constants_in(&mut binary).unwrap();

        let new_offset = extract_offset_at(&binary, 80).unwrap();
        let new_data = &binary[new_offset as usize..(new_offset as usize) + 4];

        assert_eq!(new_data, &[55, 66, 77, 88]);
    }

    #[test]
    fn test_update_last_indirect_configurable() {
        let mut binary = generate_test_binary();
        let mut configurables = setup_configurables();

        // Modify last indirect configurable data
        configurables.offsets_with_data[5].data = vec![42, 43, 44, 45];

        configurables.update_constants_in(&mut binary).unwrap();

        let new_offset = extract_offset_at(&binary, 160).unwrap();
        let new_data = &binary[new_offset as usize..(new_offset as usize) + 4];

        assert_eq!(new_data, &[42, 43, 44, 45]);
    }

    #[test]
    fn test_update_all_indirect_configurables() {
        let mut binary = generate_test_binary();
        let mut configurables = setup_configurables();

        // Modify all indirect configurable data
        configurables.offsets_with_data[3].data = vec![99, 98, 97, 96];
        configurables.offsets_with_data[4].data = vec![55, 66, 77, 88];
        configurables.offsets_with_data[5].data = vec![42, 43, 44, 45];

        configurables.update_constants_in(&mut binary).unwrap();

        // Validate all indirect configurables
        for (i, &offset) in [30, 80, 160].iter().enumerate() {
            let expected_data = match i {
                0 => &[99, 98, 97, 96],
                1 => &[55, 66, 77, 88],
                2 => &[42, 43, 44, 45],
                _ => unreachable!(),
            };

            let new_offset = extract_offset_at(&binary, offset).unwrap();
            let new_data = &binary[new_offset as usize..(new_offset as usize) + 4];

            assert_eq!(new_data, expected_data);
        }
    }
}
