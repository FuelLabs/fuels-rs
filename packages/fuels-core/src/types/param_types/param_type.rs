use itertools::chain;

use crate::{
    checked_round_up_to_word_alignment,
    types::{
        errors::{error, Result},
        param_types::EnumVariants,
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
        checked_round_up_to_word_alignment, constants::WORD_SIZE, to_named,
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
}
