pub mod codec;
pub mod traits;
pub mod types;
mod utils;

use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

use codec::{try_from_bytes, ABIDecoder, ABIEncoder, DecoderConfig};
use offsets::{extract_data_offset, extract_offset_at};
use traits::{Parameterize, Tokenizable};
use types::{param_types::ParamType, Token};
pub use utils::*;

use crate::types::errors::Result;

type OffsetWithData = (u64, Vec<u8>);

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

    pub fn try_from_direct<T: Tokenizable + Parameterize>(&self, offset: usize) -> Result<T> {
        check_binary_len(&self.binary, offset)?;

        try_from_bytes(&self.binary[offset..], self.decoder_config)
    }

    pub fn try_from_indirect<T: Tokenizable + Parameterize>(&self, offset: usize) -> Result<T> {
        let data_offset = extract_data_offset(&self.binary)?;
        let dyn_offset = extract_offset_at(&self.binary, offset)?;

        check_binary_len(&self.binary, data_offset + dyn_offset)?;

        try_from_bytes(
            &self.binary[data_offset + dyn_offset..],
            self.decoder_config,
        )
    }

    pub fn decode_direct(&self, offset: usize, param_type: &ParamType) -> Result<Token> {
        check_binary_len(&self.binary, offset)?;

        ABIDecoder::new(self.decoder_config).decode(param_type, &self.binary[offset..])
    }

    pub fn decode_indirect(&self, offset: usize, param_type: &ParamType) -> Result<Token> {
        let data_offset = extract_data_offset(&self.binary)?;
        let dyn_offset = extract_offset_at(&self.binary, offset)?;

        check_binary_len(&self.binary, data_offset + dyn_offset)?;

        ABIDecoder::new(self.decoder_config)
            .decode(param_type, &self.binary[data_offset + dyn_offset..])
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
struct OverrideConfigurables {
    direct: BTreeMap<u64, Vec<u8>>,
    indirect: BTreeMap<u64, Vec<u8>>,
}

impl OverrideConfigurables {
    fn new(direct: BTreeMap<u64, Vec<u8>>, indirect: BTreeMap<u64, Vec<u8>>) -> Self {
        Self { direct, indirect }
    }

    fn with_overrides(mut self, configurables: OverrideConfigurables) -> Self {
        self.direct.extend(configurables.direct);
        self.indirect.extend(configurables.indirect);

        self
    }

    fn update_binary(&self, binary: &mut Vec<u8>) -> Result<()> {
        self.apply_direct(binary)?;
        self.apply_indirect(binary)?;

        Ok(())
    }

    fn apply_direct(&self, binary: &mut [u8]) -> Result<()> {
        for (static_offset, data) in self.direct.iter() {
            Self::write(binary, *static_offset as usize, data)?;
        }

        Ok(())
    }

    fn apply_indirect(&self, binary: &mut Vec<u8>) -> Result<()> {
        if self.indirect.is_empty() {
            return Ok(());
        }

        let data_offset = extract_data_offset(binary)?;
        let start_of_dyn_section = self
            .dynamic_section_start(binary, data_offset)?
            .unwrap_or(binary.len());

        let mut new_dyn_section: Vec<u8> = vec![];
        let mut save_to_dyn = |data| {
            let ptr = start_of_dyn_section
                .saturating_add(new_dyn_section.len())
                .saturating_sub(data_offset);
            dbg!(&ptr);
            let ptr_encoded = ABIEncoder::default().encode(&[(ptr as u64).into_token()])?;

            new_dyn_section.extend(data);

            Result::Ok(ptr_encoded)
        };

        for (static_offset, data) in self.indirect.iter() {
            let ptr = save_to_dyn(data)?;
            Self::write(binary, *static_offset as usize, &ptr)?;
        }

        binary.truncate(start_of_dyn_section);
        binary.extend(new_dyn_section);

        Ok(())
    }

    fn write(binary: &mut [u8], offset: usize, data: &[u8]) -> Result<()> {
        check_binary_len(binary, offset + data.len())?;

        binary[offset..offset + data.len()].copy_from_slice(data);

        Ok(())
    }

    fn dynamic_section_start(&self, binary: &[u8], data_offset: usize) -> Result<Option<usize>> {
        let mut min = None;

        for (static_offset, _) in self.indirect.iter() {
            let offset =
                extract_offset_at(binary, *static_offset as usize)?.saturating_add(data_offset);

            min = min
                .map(|current_min| std::cmp::min(current_min, offset))
                .or(Some(offset));
        }

        Ok(min)
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Configurables {
    offsets_with_data: Vec<OffsetWithData>,
    indirect_offsets: BTreeSet<u64>,
}

impl Configurables {
    pub fn new(offsets_with_data: Vec<OffsetWithData>, indirect_configurables: Vec<u64>) -> Self {
        let indirect_offsets = indirect_configurables.into_iter().collect();

        Self {
            offsets_with_data,
            indirect_offsets,
        }
    }

    fn to_overrides(&self) -> OverrideConfigurables {
        let (indirect_configurables, direct_configurables) = self
            .offsets_with_data
            .iter()
            .cloned()
            .partition(|(offset, _)| self.indirect_offsets.contains(offset));

        OverrideConfigurables::new(direct_configurables, indirect_configurables)
    }

    fn read_out_indirect_configurables(&self, binary: &[u8]) -> Result<OverrideConfigurables> {
        if self.indirect_offsets.is_empty() {
            return Ok(OverrideConfigurables::default());
        }

        let data_offset = extract_data_offset(binary)?;
        let mut indirect_configurables = BTreeMap::new();

        let mut peekable_indirect_offset = self.indirect_offsets.iter().peekable();

        while let Some(current) = peekable_indirect_offset.next() {
            let data_start =
                extract_offset_at(binary, *current as usize)?.saturating_add(data_offset);

            let data_end = if let Some(next) = peekable_indirect_offset.peek() {
                extract_offset_at(binary, **next as usize)?.saturating_add(data_offset)
            } else {
                binary.len()
            };

            indirect_configurables.insert(*current, binary[data_start..data_end].to_vec());
        }

        Ok(OverrideConfigurables::new(
            BTreeMap::default(),
            indirect_configurables,
        ))
    }

    pub fn with_shifted_offsets(self, shift: i64) -> Result<Self> {
        let offsets_with_data = self
            .offsets_with_data
            .into_iter()
            .map(|(offset, data)| Ok((Self::shift_offset(offset, shift)?, data)))
            .collect::<Result<Vec<_>>>()?;

        let indirect_offsets = self
            .indirect_offsets
            .into_iter()
            .map(|offset| Self::shift_offset(offset, shift))
            .collect::<Result<BTreeSet<_>>>()?;

        Ok(Self {
            offsets_with_data,
            indirect_offsets,
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
        if self.offsets_with_data.is_empty() {
            return Ok(());
        }

        self.read_out_indirect_configurables(binary)?
            .with_overrides(self.to_overrides())
            .update_binary(binary)?;

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
        let expected_indirect_configurables = vec![15, 25];

        assert_eq!(
            shifted_configurables.offsets_with_data,
            expected_offsets_with_data
        );
        assert_eq!(
            shifted_configurables.indirect_offsets,
            expected_indirect_configurables.into_iter().collect()
        );
    }

    #[test]
    fn test_with_shifted_offsets_negative_shift() {
        let offsets_with_data = vec![(10, vec![4, 5, 6]), (30, vec![7, 8, 9])];
        let indirect_configurables = vec![30, 10];
        let configurables = Configurables::new(offsets_with_data.clone(), indirect_configurables);

        let shifted_configurables = configurables.with_shifted_offsets(-5).unwrap();
        let expected_offsets_with_data = vec![(5, vec![4, 5, 6]), (25, vec![7, 8, 9])];
        let expected_indirect_configurables = vec![5, 25];

        assert_eq!(
            shifted_configurables.offsets_with_data,
            expected_offsets_with_data
        );
        assert_eq!(
            shifted_configurables.indirect_offsets,
            expected_indirect_configurables.into_iter().collect()
        );
    }

    #[test]
    fn test_with_shifted_offsets_zero_shift() {
        let offsets_with_data = vec![(20, vec![7, 8, 9]), (40, vec![10, 11, 12])];
        let indirect_configurables = vec![40, 20];
        let configurables = Configurables::new(offsets_with_data.clone(), indirect_configurables);

        let shifted_configurables = configurables.with_shifted_offsets(0).unwrap();
        let expected_offsets_with_data = offsets_with_data;
        let expected_indirect_configurables = vec![20, 40];

        assert_eq!(
            shifted_configurables.offsets_with_data,
            expected_offsets_with_data
        );
        assert_eq!(
            shifted_configurables.indirect_offsets,
            expected_indirect_configurables.into_iter().collect()
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

    const TEST_BINARY: [u8; 55] = [
        0, 1, 2, 3, 4, 5, 6, 7, 0, 0, 0, 0, 0, 0, 0, 16, 17, 18, 19, 20, 21, 22, 0, 0, 0, 0, 0, 0,
        0, 30, 0, 0, 0, 0, 0, 0, 0, 33, 0, 0, 0, 0, 0, 0, 0, 36, 50, 51, 52, 53, 54, 55, 56, 57,
        58,
    ];

    fn setup_configurables() -> Configurables {
        let offsets_with_data = vec![(16, vec![34, 36]), (18, vec![38, 40]), (20, vec![42, 44])];

        let indirect_configurables = vec![22, 30, 38];

        Configurables::new(offsets_with_data, indirect_configurables)
    }

    #[test]
    fn try_from_direct() {
        let configurables_reader = ConfigurablesReader::load(TEST_BINARY.to_vec());
        let value: u16 = configurables_reader.try_from_direct(16).unwrap();

        assert_eq!(4370, value);
    }

    #[test]
    fn try_from_indirect() {
        let configurables_reader = ConfigurablesReader::load(TEST_BINARY.to_vec());
        let value: [u8; 3] = configurables_reader.try_from_indirect(22).unwrap();

        assert_eq!([50, 51, 52], value);
    }

    #[test]
    fn decode_direct() {
        let configurables_reader = ConfigurablesReader::load(TEST_BINARY.to_vec());
        let token = configurables_reader
            .decode_direct(16, &u16::param_type())
            .unwrap();

        assert_eq!(4370u16.into_token(), token);
    }

    #[test]
    fn decode_indirect() {
        let configurables_reader = ConfigurablesReader::load(TEST_BINARY.to_vec());
        let token = configurables_reader
            .decode_indirect(22, &<[u8; 3]>::param_type())
            .unwrap();

        assert_eq!([50u8, 51, 52].into_token(), token);
    }

    #[test]
    fn update_first_indirect_configurable_less_data() {
        let mut binary = TEST_BINARY.to_vec();
        let mut configurables = setup_configurables();

        // Modify first indirect configurable with less data
        configurables.offsets_with_data.push((22, vec![100, 102]));

        configurables.update_constants_in(&mut binary).unwrap();

        let expected_binary: [u8; 54] = [
            0, 1, 2, 3, 4, 5, 6, 7, 0, 0, 0, 0, 0, 0, 0, 16, 34, 36, 38, 40, 42, 44, 0, 0, 0, 0, 0,
            0, 0, 30, 0, 0, 0, 0, 0, 0, 0, 32, 0, 0, 0, 0, 0, 0, 0, 35, 100, 102, 53, 54, 55, 56,
            57, 58,
        ];

        pretty_assertions::assert_eq!(&expected_binary, binary.as_slice());
    }

    #[test]
    fn update_first_indirect_configurable_more_data() {
        let mut binary = TEST_BINARY.to_vec();
        let mut configurables = setup_configurables();

        // Modify first indirect configurable with more data
        configurables
            .offsets_with_data
            .push((22, vec![100, 102, 103, 104]));

        configurables.update_constants_in(&mut binary).unwrap();

        let expected_binary: [u8; 56] = [
            0, 1, 2, 3, 4, 5, 6, 7, 0, 0, 0, 0, 0, 0, 0, 16, 34, 36, 38, 40, 42, 44, 0, 0, 0, 0, 0,
            0, 0, 30, 0, 0, 0, 0, 0, 0, 0, 34, 0, 0, 0, 0, 0, 0, 0, 37, 100, 102, 103, 104, 53, 54,
            55, 56, 57, 58,
        ];

        pretty_assertions::assert_eq!(&expected_binary, binary.as_slice());
    }

    #[test]
    fn update_second_indirect_configurable_less_data() {
        let mut binary = TEST_BINARY.to_vec();
        let mut configurables = setup_configurables();

        // Modify second indirect configurable with less data
        configurables.offsets_with_data.push((30, vec![106, 108]));

        configurables.update_constants_in(&mut binary).unwrap();

        let expected_binary: [u8; 54] = [
            0, 1, 2, 3, 4, 5, 6, 7, 0, 0, 0, 0, 0, 0, 0, 16, 34, 36, 38, 40, 42, 44, 0, 0, 0, 0, 0,
            0, 0, 30, 0, 0, 0, 0, 0, 0, 0, 33, 0, 0, 0, 0, 0, 0, 0, 35, 50, 51, 52, 106, 108, 56,
            57, 58,
        ];

        pretty_assertions::assert_eq!(&expected_binary, binary.as_slice());
    }

    #[test]
    fn update_second_indirect_configurable_more_data() {
        let mut binary = TEST_BINARY.to_vec();
        let mut configurables = setup_configurables();

        // Modify second indirect configurable with more data
        configurables
            .offsets_with_data
            .push((30, vec![106, 108, 110, 112]));

        configurables.update_constants_in(&mut binary).unwrap();

        let expected_binary: [u8; 56] = [
            0, 1, 2, 3, 4, 5, 6, 7, 0, 0, 0, 0, 0, 0, 0, 16, 34, 36, 38, 40, 42, 44, 0, 0, 0, 0, 0,
            0, 0, 30, 0, 0, 0, 0, 0, 0, 0, 33, 0, 0, 0, 0, 0, 0, 0, 37, 50, 51, 52, 106, 108, 110,
            112, 56, 57, 58,
        ];

        pretty_assertions::assert_eq!(&expected_binary, binary.as_slice());
    }

    #[test]
    fn update_last_indirect_configurable_less_data() {
        let mut binary = TEST_BINARY.to_vec();
        let mut configurables = setup_configurables();

        // Modify last indirect configurable with less data
        configurables.offsets_with_data.push((38, vec![112, 114]));

        configurables.update_constants_in(&mut binary).unwrap();

        let expected_binary: [u8; 54] = [
            0, 1, 2, 3, 4, 5, 6, 7, 0, 0, 0, 0, 0, 0, 0, 16, 34, 36, 38, 40, 42, 44, 0, 0, 0, 0, 0,
            0, 0, 30, 0, 0, 0, 0, 0, 0, 0, 33, 0, 0, 0, 0, 0, 0, 0, 36, 50, 51, 52, 53, 54, 55,
            112, 114,
        ];

        pretty_assertions::assert_eq!(&expected_binary, binary.as_slice());
    }

    #[test]
    fn update_last_indirect_configurable_more_data() {
        let mut binary = TEST_BINARY.to_vec();
        let mut configurables = setup_configurables();

        // Modify last indirect configurable with more data
        configurables
            .offsets_with_data
            .push((38, vec![112, 114, 116, 118]));

        configurables.update_constants_in(&mut binary).unwrap();

        let expected_binary: [u8; 56] = [
            0, 1, 2, 3, 4, 5, 6, 7, 0, 0, 0, 0, 0, 0, 0, 16, 34, 36, 38, 40, 42, 44, 0, 0, 0, 0, 0,
            0, 0, 30, 0, 0, 0, 0, 0, 0, 0, 33, 0, 0, 0, 0, 0, 0, 0, 36, 50, 51, 52, 53, 54, 55,
            112, 114, 116, 118,
        ];

        pretty_assertions::assert_eq!(&expected_binary, binary.as_slice());
    }

    #[test]
    fn update_all_indirect_configurables() {
        let mut binary = TEST_BINARY.to_vec();
        let mut configurables = setup_configurables();

        // Modify all indirect configurables
        configurables.offsets_with_data.push((22, vec![100, 101]));
        configurables
            .offsets_with_data
            .push((30, vec![102, 103, 104, 105]));
        configurables
            .offsets_with_data
            .push((38, vec![106, 107, 108]));

        configurables.update_constants_in(&mut binary).unwrap();

        let expected_binary: [u8; 55] = [
            0, 1, 2, 3, 4, 5, 6, 7, 0, 0, 0, 0, 0, 0, 0, 16, 34, 36, 38, 40, 42, 44, 0, 0, 0, 0, 0,
            0, 0, 30, 0, 0, 0, 0, 0, 0, 0, 32, 0, 0, 0, 0, 0, 0, 0, 36, 100, 101, 102, 103, 104,
            105, 106, 107, 108,
        ];

        pretty_assertions::assert_eq!(&expected_binary, binary.as_slice());
    }
}
