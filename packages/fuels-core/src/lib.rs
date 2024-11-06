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

    pub fn with_shifted_offsets(self, shift: i64) -> Result<Self> {
        let new_offsets_with_data = self
            .offsets_with_data
            .into_iter()
            .map(|(offset, data)| {
                let new_offset = if shift.is_negative() {
                    offset.checked_sub(shift.unsigned_abs())
                } else {
                    offset.checked_add(shift.unsigned_abs())
                };

                let new_offset = new_offset.ok_or_else(|| {
                    crate::error!(
                        Other,
                        "Overflow occurred while shifting offset: {offset} + {shift}"
                    )
                })?;

                Ok((new_offset, data.clone()))
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(Self {
            offsets_with_data: new_offsets_with_data,
        })
    }

    pub fn update_constants_in(&self, binary: &mut [u8]) {
        for (offset, data) in &self.offsets_with_data {
            let offset = *offset as usize;
            binary[offset..offset + data.len()].copy_from_slice(data)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_with_shifted_offsets_positive_shift() {
        let offsets_with_data = vec![(10u64, vec![1, 2, 3])];
        let configurables = Configurables::new(offsets_with_data.clone());
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
        let configurables = Configurables::new(offsets_with_data.clone());
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
        let configurables = Configurables::new(offsets_with_data.clone());
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
        let configurables = Configurables::new(offsets_with_data);
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
        let configurables = Configurables::new(offsets_with_data);
        let result = configurables.with_shifted_offsets(-10);
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e
                .to_string()
                .contains("Overflow occurred while shifting offset"));
        }
    }
}
