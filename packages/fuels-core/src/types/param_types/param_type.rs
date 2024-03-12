use itertools::chain;

use crate::{
    checked_round_up_to_word_alignment,
    types::{
        errors::{error, Result},
        param_types::{debug_with_depth::DebugWithDepth, EnumVariants},
    },
};

pub type NamedParamType = (String, ParamType);

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ParamType {
    Unit,
    Bool,
    U8,
    U16,
    U32,
    U64,
    U128,
    U256,
    B256,
    Bytes,
    String,
    RawSlice,
    StringArray(usize),
    StringSlice,
    Tuple(Vec<ParamType>),
    Array(Box<ParamType>, usize),
    Vector(Box<ParamType>),
    Struct {
        name: String,
        fields: Vec<NamedParamType>,
        generics: Vec<ParamType>,
    },
    Enum {
        name: String,
        enum_variants: EnumVariants,
        generics: Vec<ParamType>,
    },
}

pub enum ReturnLocation {
    Return,
    ReturnData,
}

impl ParamType {
    // Depending on the type, the returned value will be stored
    // either in `Return` or `ReturnData`.
    pub fn get_return_location(&self) -> ReturnLocation {
        match self {
            Self::Unit | Self::U8 | Self::U16 | Self::U32 | Self::U64 | Self::Bool => {
                ReturnLocation::Return
            }

            _ => ReturnLocation::ReturnData,
        }
    }

    /// Given a [ParamType], return the number of elements of that [ParamType] that can fit in
    /// `available_bytes`: it is the length of the corresponding heap type.
    pub fn calculate_num_of_elements(
        param_type: &ParamType,
        available_bytes: usize,
    ) -> Result<usize> {
        let memory_size = param_type.compute_encoding_in_bytes()?;
        if memory_size == 0 {
            return Err(error!(
                Codec,
                "cannot calculate the number of elements because the type is zero-sized"
            ));
        }

        let remainder = available_bytes % memory_size;
        if remainder != 0 {
            return Err(error!(
                Codec,
                "{remainder} extra bytes detected while decoding heap type"
            ));
        }
        let num_of_elements = available_bytes
            .checked_div(memory_size)
            .ok_or_else(|| error!(Codec, "type {param_type:?} has a memory_size of 0"))?;

        Ok(num_of_elements)
    }

    pub fn children_need_extra_receipts(&self) -> bool {
        match self {
            ParamType::Array(inner, _) | ParamType::Vector(inner) => {
                inner.is_extra_receipt_needed(false)
            }
            ParamType::Struct { fields, .. } => fields
                .iter()
                .any(|(_, param_type)| param_type.is_extra_receipt_needed(false)),
            ParamType::Enum { enum_variants, .. } => enum_variants
                .param_types()
                .any(|param_type| param_type.is_extra_receipt_needed(false)),
            ParamType::Tuple(inner_types) => inner_types
                .iter()
                .any(|param_type| param_type.is_extra_receipt_needed(false)),
            _ => false,
        }
    }

    pub fn validate_is_decodable(&self, max_depth: usize) -> Result<()> {
        if let ParamType::Enum { enum_variants, .. } = self {
            let grandchildren_need_receipts = enum_variants
                .param_types()
                .any(|child| child.children_need_extra_receipts());
            if grandchildren_need_receipts {
                return Err(error!(
                    Codec,
                    "enums currently support only one level deep heap types"
                ));
            }

            let num_of_children_needing_receipts = enum_variants
                .param_types()
                .filter(|param_type| param_type.is_extra_receipt_needed(false))
                .count();
            if num_of_children_needing_receipts > 1 {
                return Err(error!(
                    Codec,
                    "enums currently support only one heap-type variant. Found: \
                        {num_of_children_needing_receipts}"
                ));
            }
        } else if self.children_need_extra_receipts() {
            return Err(error!(
                Codec,
                "type `{:?}` is not decodable: nested heap types are currently not \
                    supported except in enums",
                DebugWithDepth::new(self, max_depth)
            ));
        }
        self.compute_encoding_in_bytes()?;

        Ok(())
    }

    pub fn is_extra_receipt_needed(&self, top_level_type: bool) -> bool {
        match self {
            ParamType::Vector(_) | ParamType::Bytes | ParamType::String => true,
            ParamType::Array(inner, _) => inner.is_extra_receipt_needed(false),
            ParamType::Struct {
                fields, generics, ..
            } => chain!(fields.iter().map(|(_, param_type)| param_type), generics,)
                .any(|param_type| param_type.is_extra_receipt_needed(false)),
            ParamType::Enum {
                enum_variants,
                generics,
                ..
            } => chain!(enum_variants.param_types(), generics)
                .any(|param_type| param_type.is_extra_receipt_needed(false)),
            ParamType::Tuple(elements) => elements
                .iter()
                .any(|param_type| param_type.is_extra_receipt_needed(false)),
            ParamType::RawSlice | ParamType::StringSlice => !top_level_type,
            _ => false,
        }
    }

    /// Compute the inner memory size of a containing heap type (`Bytes` or `Vec`s).
    pub fn heap_inner_element_size(&self, top_level_type: bool) -> Result<Option<usize>> {
        let heap_bytes_size = match &self {
            ParamType::Vector(inner_param_type) => {
                Some(inner_param_type.compute_encoding_in_bytes()?)
            }
            // `Bytes` type is byte-packed in the VM, so it's the size of an u8
            ParamType::Bytes | ParamType::String => Some(std::mem::size_of::<u8>()),
            ParamType::StringSlice if !top_level_type => {
                Some(ParamType::U8.compute_encoding_in_bytes()?)
            }
            ParamType::RawSlice if !top_level_type => {
                Some(ParamType::U64.compute_encoding_in_bytes()?)
            }
            _ => None,
        };
        Ok(heap_bytes_size)
    }

    /// Calculates the number of bytes the VM expects this parameter to be encoded in.
    pub fn compute_encoding_in_bytes(&self) -> Result<usize> {
        let overflow_error = || {
            error!(
                Codec,
                "reached overflow while computing encoding size for {:?}", self
            )
        };
        match &self {
            ParamType::Unit | ParamType::U8 | ParamType::Bool => Ok(1),
            ParamType::U16 | ParamType::U32 | ParamType::U64 => Ok(8),
            ParamType::U128 | ParamType::RawSlice | ParamType::StringSlice => Ok(16),
            ParamType::U256 | ParamType::B256 => Ok(32),
            ParamType::Vector(_) | ParamType::Bytes | ParamType::String => Ok(24),
            ParamType::Array(param, count) => param
                .compute_encoding_in_bytes()?
                .checked_mul(*count)
                .ok_or_else(overflow_error),
            ParamType::StringArray(len) => {
                checked_round_up_to_word_alignment(*len).map_err(|_| overflow_error())
            }
            ParamType::Tuple(fields) => fields.iter().try_fold(0, |a: usize, param_type| {
                let size =
                    checked_round_up_to_word_alignment(param_type.compute_encoding_in_bytes()?)?;
                a.checked_add(size).ok_or_else(overflow_error)
            }),
            ParamType::Struct { fields, .. } => fields
                .iter()
                .map(|(_, param_type)| param_type)
                .try_fold(0, |a: usize, param_type| {
                    let size = checked_round_up_to_word_alignment(
                        param_type.compute_encoding_in_bytes()?,
                    )?;
                    a.checked_add(size).ok_or_else(overflow_error)
                }),
            ParamType::Enum { enum_variants, .. } => enum_variants
                .compute_enum_width_in_bytes()
                .map_err(|_| overflow_error()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        checked_round_up_to_word_alignment, codec::DecoderConfig, constants::WORD_SIZE, to_named,
        types::param_types::ParamType,
    };

    const WIDTH_OF_B256: usize = 32;
    const WIDTH_OF_U32: usize = 8;
    const WIDTH_OF_BOOL: usize = 1;

    #[test]
    fn calculate_num_of_elements() -> Result<()> {
        let failing_param_type = ParamType::Array(Box::new(ParamType::U16), usize::MAX);
        assert!(ParamType::calculate_num_of_elements(&failing_param_type, 0)
            .unwrap_err()
            .to_string()
            .contains("reached overflow"));

        let zero_sized_type = ParamType::Array(Box::new(ParamType::StringArray(0)), 1000);
        assert!(ParamType::calculate_num_of_elements(&zero_sized_type, 0)
            .unwrap_err()
            .to_string()
            .contains("the type is zero-sized"));

        assert!(ParamType::calculate_num_of_elements(&ParamType::U16, 9)
            .unwrap_err()
            .to_string()
            .contains("1 extra bytes detected while decoding heap type"));

        Ok(())
    }

    #[test]
    fn array_size_dependent_on_num_of_elements() {
        const NUM_ELEMENTS: usize = 11;
        let param = ParamType::Array(Box::new(ParamType::B256), NUM_ELEMENTS);

        let width = param.compute_encoding_in_bytes().unwrap();

        let expected = NUM_ELEMENTS * WIDTH_OF_B256;
        assert_eq!(expected, width);
    }

    #[test]
    fn string_size_dependent_on_num_of_elements() {
        const NUM_ASCII_CHARS: usize = 9;
        let param = ParamType::StringArray(NUM_ASCII_CHARS);

        let width = param.compute_encoding_in_bytes().unwrap();

        assert_eq!(16, width);
    }

    #[test]
    fn structs_are_all_elements_combined_with_padding() -> Result<()> {
        let inner_struct = ParamType::Struct {
            name: "".to_string(),
            fields: to_named(&[ParamType::U32, ParamType::U32]),
            generics: vec![],
        };

        let a_struct = ParamType::Struct {
            name: "".to_string(),
            fields: to_named(&[ParamType::B256, ParamType::Bool, inner_struct]),
            generics: vec![],
        };

        let width = a_struct.compute_encoding_in_bytes().unwrap();

        const INNER_STRUCT_WIDTH: usize = WIDTH_OF_U32 * 2;
        let expected_width: usize =
            WIDTH_OF_B256 + checked_round_up_to_word_alignment(WIDTH_OF_BOOL)? + INNER_STRUCT_WIDTH;
        assert_eq!(expected_width, width);
        Ok(())
    }

    #[test]
    fn enums_are_as_big_as_their_biggest_variant_plus_a_word() -> Result<()> {
        let fields = to_named(&[ParamType::B256]);
        let inner_struct = ParamType::Struct {
            name: "".to_string(),
            fields,
            generics: vec![],
        };
        let types = to_named(&[ParamType::U32, inner_struct]);
        let param = ParamType::Enum {
            name: "".to_string(),
            enum_variants: EnumVariants::new(types)?,
            generics: vec![],
        };

        let width = param.compute_encoding_in_bytes().unwrap();

        const INNER_STRUCT_SIZE: usize = WIDTH_OF_B256;
        const EXPECTED_WIDTH: usize = INNER_STRUCT_SIZE + WORD_SIZE;
        assert_eq!(EXPECTED_WIDTH, width);
        Ok(())
    }

    #[test]
    fn tuples_are_just_all_elements_combined() {
        let inner_tuple = ParamType::Tuple(vec![ParamType::B256]);
        let param = ParamType::Tuple(vec![ParamType::U32, inner_tuple]);

        let width = param.compute_encoding_in_bytes().unwrap();

        const INNER_TUPLE_WIDTH: usize = WIDTH_OF_B256;
        const EXPECTED_WIDTH: usize = WIDTH_OF_U32 + INNER_TUPLE_WIDTH;
        assert_eq!(EXPECTED_WIDTH, width);
    }

    #[test]
    fn test_compute_encoding_in_bytes_overflows() -> Result<()> {
        let overflows = |p: ParamType| {
            let error = p.compute_encoding_in_bytes().unwrap_err();
            let overflow_error = error!(
                Codec,
                "reached overflow while computing encoding size for {:?}", p
            );
            assert_eq!(error.to_string(), overflow_error.to_string());
        };
        let tuple_with_fields_too_wide = ParamType::Tuple(vec![
            ParamType::StringArray(12514849900987264429),
            ParamType::StringArray(7017071859781709229),
        ]);
        overflows(tuple_with_fields_too_wide);

        let struct_with_fields_too_wide = ParamType::Struct {
            name: "".to_string(),
            fields: to_named(&[
                ParamType::StringArray(12514849900987264429),
                ParamType::StringArray(7017071859781709229),
            ]),
            generics: vec![],
        };
        overflows(struct_with_fields_too_wide);

        let enum_with_variants_too_wide = ParamType::Enum {
            name: "".to_string(),
            enum_variants: EnumVariants::new(to_named(&[ParamType::StringArray(usize::MAX - 8)]))?,
            generics: vec![],
        };
        overflows(enum_with_variants_too_wide);

        let array_too_big = ParamType::Array(Box::new(ParamType::U64), usize::MAX);
        overflows(array_too_big);

        let string_array_too_big = ParamType::StringArray(usize::MAX);
        overflows(string_array_too_big);
        Ok(())
    }

    #[test]
    fn validate_is_decodable_simple_types() -> Result<()> {
        let max_depth = DecoderConfig::default().max_depth;
        assert!(ParamType::U8.validate_is_decodable(max_depth).is_ok());
        assert!(ParamType::U16.validate_is_decodable(max_depth).is_ok());
        assert!(ParamType::U32.validate_is_decodable(max_depth).is_ok());
        assert!(ParamType::U64.validate_is_decodable(max_depth).is_ok());
        assert!(ParamType::U128.validate_is_decodable(max_depth).is_ok());
        assert!(ParamType::U256.validate_is_decodable(max_depth).is_ok());
        assert!(ParamType::Bool.validate_is_decodable(max_depth).is_ok());
        assert!(ParamType::B256.validate_is_decodable(max_depth).is_ok());
        assert!(ParamType::Unit.validate_is_decodable(max_depth).is_ok());
        assert!(ParamType::StringSlice
            .validate_is_decodable(max_depth)
            .is_ok());
        assert!(ParamType::StringArray(10)
            .validate_is_decodable(max_depth)
            .is_ok());
        assert!(ParamType::RawSlice.validate_is_decodable(max_depth).is_ok());
        assert!(ParamType::Bytes.validate_is_decodable(max_depth).is_ok());
        assert!(ParamType::String.validate_is_decodable(max_depth).is_ok());
        Ok(())
    }

    #[test]
    fn validate_is_decodable_enum_containing_bytes() -> Result<()> {
        let max_depth = DecoderConfig::default().max_depth;
        let can_be_decoded = |p: ParamType| p.validate_is_decodable(max_depth).is_ok();
        let param_types_containing_bytes = vec![ParamType::Bytes, ParamType::U64, ParamType::Bool];
        let param_types_no_bytes = vec![ParamType::U64, ParamType::U32];
        let variants_no_bytes_type = EnumVariants::new(to_named(&param_types_no_bytes))?;
        let variants_one_bytes_type = EnumVariants::new(to_named(&param_types_containing_bytes))?;
        let variants_two_bytes_type =
            EnumVariants::new(to_named(&[ParamType::Bytes, ParamType::Bytes]))?;

        can_be_decoded(ParamType::Enum {
            name: "".to_string(),
            enum_variants: variants_no_bytes_type.clone(),
            generics: param_types_no_bytes.clone(),
        });

        can_be_decoded(ParamType::Enum {
            name: "".to_string(),
            enum_variants: variants_one_bytes_type.clone(),
            generics: param_types_no_bytes.clone(),
        });

        let expected =
            "codec: enums currently support only one heap-type variant. Found: 2".to_string();

        assert_eq!(
            ParamType::Enum {
                name: "".to_string(),
                enum_variants: variants_two_bytes_type.clone(),
                generics: param_types_no_bytes.clone(),
            }
            .validate_is_decodable(max_depth)
            .expect_err("should not be decodable")
            .to_string(),
            expected
        );

        can_be_decoded(ParamType::Enum {
            name: "".to_string(),
            enum_variants: variants_no_bytes_type,
            generics: param_types_containing_bytes.clone(),
        });

        can_be_decoded(ParamType::Enum {
            name: "".to_string(),
            enum_variants: variants_one_bytes_type,
            generics: param_types_containing_bytes.clone(),
        });

        let expected =
            "codec: enums currently support only one heap-type variant. Found: 2".to_string();

        assert_eq!(
            ParamType::Enum {
                name: "".to_string(),
                enum_variants: variants_two_bytes_type.clone(),
                generics: param_types_containing_bytes.clone(),
            }
            .validate_is_decodable(max_depth)
            .expect_err("should not be decodable")
            .to_string(),
            expected
        );

        Ok(())
    }

    #[test]
    fn validate_is_decodable_enum_containing_string() -> Result<()> {
        let max_depth = DecoderConfig::default().max_depth;
        let can_be_decoded = |p: ParamType| p.validate_is_decodable(max_depth).is_ok();
        let param_types_containing_string = vec![ParamType::Bytes, ParamType::U64, ParamType::Bool];
        let param_types_no_string = vec![ParamType::U64, ParamType::U32];
        let variants_no_string_type = EnumVariants::new(to_named(&param_types_no_string))?;
        let variants_one_string_type = EnumVariants::new(to_named(&param_types_containing_string))?;
        let variants_two_string_type =
            EnumVariants::new(to_named(&[ParamType::Bytes, ParamType::Bytes]))?;

        can_be_decoded(ParamType::Enum {
            name: "".to_string(),
            enum_variants: variants_no_string_type.clone(),
            generics: param_types_no_string.clone(),
        });

        can_be_decoded(ParamType::Enum {
            name: "".to_string(),
            enum_variants: variants_one_string_type.clone(),
            generics: param_types_no_string.clone(),
        });

        let expected =
            "codec: enums currently support only one heap-type variant. Found: 2".to_string();

        assert_eq!(
            ParamType::Enum {
                name: "".to_string(),
                enum_variants: variants_two_string_type.clone(),
                generics: param_types_no_string.clone(),
            }
            .validate_is_decodable(1)
            .expect_err("should not be decodable")
            .to_string(),
            expected
        );

        can_be_decoded(ParamType::Enum {
            name: "".to_string(),
            enum_variants: variants_no_string_type,
            generics: param_types_containing_string.clone(),
        });

        can_be_decoded(ParamType::Enum {
            name: "".to_string(),
            enum_variants: variants_one_string_type,
            generics: param_types_containing_string.clone(),
        });

        let expected =
            "codec: enums currently support only one heap-type variant. Found: 2".to_string();
        assert_eq!(
            ParamType::Enum {
                name: "".to_string(),
                enum_variants: variants_two_string_type.clone(),
                generics: param_types_containing_string.clone(),
            }
            .validate_is_decodable(1)
            .expect_err("should not be decodable")
            .to_string(),
            expected
        );

        Ok(())
    }

    #[test]
    fn validate_is_decodable_enum_containing_vector() -> Result<()> {
        let max_depth = DecoderConfig::default().max_depth;
        let can_be_decoded = |p: ParamType| p.validate_is_decodable(max_depth).is_ok();
        let param_types_containing_vector = vec![
            ParamType::Vector(Box::new(ParamType::Bool)),
            ParamType::U64,
            ParamType::Bool,
        ];
        let param_types_no_vector = vec![ParamType::U64, ParamType::U32];
        let variants_no_vector_type = EnumVariants::new(to_named(&param_types_no_vector))?;
        let variants_one_vector_type = EnumVariants::new(to_named(&param_types_containing_vector))?;
        let variants_two_vector_type = EnumVariants::new(to_named(&[
            ParamType::Vector(Box::new(ParamType::U8)),
            ParamType::Vector(Box::new(ParamType::U16)),
        ]))?;

        can_be_decoded(ParamType::Enum {
            name: "".to_string(),
            enum_variants: variants_no_vector_type.clone(),
            generics: param_types_no_vector.clone(),
        });

        can_be_decoded(ParamType::Enum {
            name: "".to_string(),
            enum_variants: variants_one_vector_type.clone(),
            generics: param_types_no_vector.clone(),
        });

        let expected =
            "codec: enums currently support only one heap-type variant. Found: 2".to_string();
        assert_eq!(
            ParamType::Enum {
                name: "".to_string(),
                enum_variants: variants_two_vector_type.clone(),
                generics: param_types_no_vector.clone(),
            }
            .validate_is_decodable(max_depth)
            .expect_err("should not be decodable")
            .to_string(),
            expected
        );
        can_be_decoded(ParamType::Enum {
            name: "".to_string(),
            enum_variants: variants_no_vector_type,
            generics: param_types_containing_vector.clone(),
        });
        can_be_decoded(ParamType::Enum {
            name: "".to_string(),
            enum_variants: variants_one_vector_type,
            generics: param_types_containing_vector.clone(),
        });
        let expected =
            "codec: enums currently support only one heap-type variant. Found: 2".to_string();
        assert_eq!(
            ParamType::Enum {
                name: "".to_string(),
                enum_variants: variants_two_vector_type.clone(),
                generics: param_types_containing_vector.clone(),
            }
            .validate_is_decodable(max_depth)
            .expect_err("should not be decodable")
            .to_string(),
            expected
        );

        Ok(())
    }
}
