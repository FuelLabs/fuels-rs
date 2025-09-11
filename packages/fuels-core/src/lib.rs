pub mod codec;
pub mod traits;
pub mod types;
mod utils;

pub use utils::*;

use crate::types::errors::Result;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Configurable {
    /// The offset (in bytes) within the binary where the data is located.
    pub offset: u64,
    /// The data related to the configurable.
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Configurables {
    pub offsets_with_data: Vec<Configurable>,
}

impl Configurables {
    pub fn new(offsets_with_data: Vec<Configurable>) -> Self {
        Self { offsets_with_data }
    }

    pub fn with_shifted_offsets(self, shift: i64) -> Result<Self> {
        let new_offsets_with_data = self
            .offsets_with_data
            .into_iter()
            .map(|c| {
                let new_offset = if shift.is_negative() {
                    c.offset.checked_sub(shift.unsigned_abs())
                } else {
                    c.offset.checked_add(shift.unsigned_abs())
                };

                let new_offset = new_offset.ok_or_else(|| {
                    crate::error!(
                        Other,
                        "Overflow occurred while shifting offset: {} + {shift}",
                        c.offset
                    )
                })?;

                Ok(Configurable {
                    offset: new_offset,
                    data: c.data,
                })
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(Self {
            offsets_with_data: new_offsets_with_data,
        })
    }

    pub fn update_constants_in(&self, binary: &mut [u8]) {
        for c in &self.offsets_with_data {
            let offset = c.offset as usize;
            binary[offset..offset + c.data.len()].copy_from_slice(&c.data)
        }
    }
}

impl From<Configurables> for Vec<Configurable> {
    fn from(config: Configurables) -> Vec<Configurable> {
        config.offsets_with_data.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_with_shifted_offsets_positive_shift() {
        let offsets_with_data = vec![Configurable {
            offset: 10u64,
            data: vec![1, 2, 3],
        }];
        let configurables = Configurables::new(offsets_with_data.clone());
        let shifted_configurables = configurables.with_shifted_offsets(5).unwrap();
        let expected_offsets_with_data = vec![Configurable {
            offset: 15u64,
            data: vec![1, 2, 3],
        }];
        assert_eq!(
            shifted_configurables.offsets_with_data,
            expected_offsets_with_data
        );
    }

    #[test]
    fn test_with_shifted_offsets_negative_shift() {
        let offsets_with_data = vec![Configurable {
            offset: 10u64,
            data: vec![4, 5, 6],
        }];
        let configurables = Configurables::new(offsets_with_data.clone());
        let shifted_configurables = configurables.with_shifted_offsets(-5).unwrap();
        let expected_offsets_with_data = vec![Configurable {
            offset: 5u64,
            data: vec![4, 5, 6],
        }];
        assert_eq!(
            shifted_configurables.offsets_with_data,
            expected_offsets_with_data
        );
    }

    #[test]
    fn test_with_shifted_offsets_zero_shift() {
        let offsets_with_data = vec![Configurable {
            offset: 20u64,
            data: vec![7, 8, 9],
        }];
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
        let offsets_with_data = vec![Configurable {
            offset: u64::MAX - 1,
            data: vec![1, 2, 3],
        }];
        let configurables = Configurables::new(offsets_with_data);
        let result = configurables.with_shifted_offsets(10);
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(
                e.to_string()
                    .contains("Overflow occurred while shifting offset")
            );
        }
    }

    #[test]
    fn test_with_shifted_offsets_underflow() {
        let offsets_with_data = vec![Configurable {
            offset: 5u64,
            data: vec![4, 5, 6],
        }];
        let configurables = Configurables::new(offsets_with_data);
        let result = configurables.with_shifted_offsets(-10);
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(
                e.to_string()
                    .contains("Overflow occurred while shifting offset")
            );
        }
    }
}
