use crate::constants::{ENUM_DISCRIMINANT_WORD_WIDTH, WORD_SIZE};
use crate::errors::Error;
use core::fmt;
use strum_macros::EnumString;
use thiserror::Error as ThisError;

#[derive(Debug, Clone, EnumString, PartialEq, Eq)]
#[strum(ascii_case_insensitive)]
pub enum ParamType {
    U8,
    U16,
    U32,
    U64,
    Bool,
    Byte,
    B256,
    // The Unit paramtype is used for unit variants in Enums. The corresponding type field is `()`,
    // similar to Rust.
    Unit,
    Array(Box<ParamType>, usize),
    #[strum(serialize = "str")]
    String(usize),
    #[strum(disabled)]
    Struct(Vec<ParamType>),
    #[strum(disabled)]
    Enum(EnumVariants),
    Tuple(Vec<ParamType>),
    Generic(String),
}

impl Default for ParamType {
    fn default() -> Self {
        ParamType::U8
    }
}

pub enum ReturnLocation {
    Return,
    ReturnData,
}

impl ParamType {
    // Depending on the type, the returned value will be stored
    // either in `Return` or `ReturnData`. For more information,
    // see https://github.com/FuelLabs/sway/issues/1368.
    pub fn get_return_location(&self) -> ReturnLocation {
        match self {
            Self::Unit | Self::U8 | Self::U16 | Self::U32 | Self::U64 | Self::Bool => {
                ReturnLocation::Return
            }

            _ => ReturnLocation::ReturnData,
        }
    }

    /// Calculates the number of `WORD`s the VM expects this parameter to be encoded in.
    pub fn compute_encoding_width(&self) -> usize {
        const fn count_words(bytes: usize) -> usize {
            let q = bytes / WORD_SIZE;
            let r = bytes % WORD_SIZE;
            match r == 0 {
                true => q,
                false => q + 1,
            }
        }

        match &self {
            ParamType::Unit
            | ParamType::U8
            | ParamType::U16
            | ParamType::U32
            | ParamType::U64
            | ParamType::Bool
            | ParamType::Byte => 1,
            ParamType::B256 => 4,
            ParamType::Array(param, count) => param.compute_encoding_width() * count,
            ParamType::String(len) => count_words(*len),
            ParamType::Struct(params) => params.iter().map(|p| p.compute_encoding_width()).sum(),
            ParamType::Enum(variants) => variants.compute_encoding_width_of_enum(),
            ParamType::Tuple(params) => params.iter().map(|p| p.compute_encoding_width()).sum(),
            ParamType::Generic(_) => {
                panic!("Generic parameters are not resolved and as such don't have a size!")
            }
        }
    }
}

impl fmt::Display for ParamType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ParamType::String(size) => {
                let t = format!("String({})", size);
                write!(f, "{}", t)
            }
            ParamType::Array(t, size) => {
                let boxed_type_str = format!("Box::new(ParamType::{})", t);
                let arr_str = format!("Array({},{})", boxed_type_str, size);
                write!(f, "{}", arr_str)
            }
            ParamType::Struct(inner) => {
                let inner_strings: Vec<String> =
                    inner.iter().map(|p| format!("ParamType::{}", p)).collect();

                let s = format!("Struct(vec![{}])", inner_strings.join(","));
                write!(f, "{}", s)
            }
            ParamType::Enum(variants) => {
                let inner_strings: Vec<String> = variants
                    .param_types()
                    .iter()
                    .map(|p| format!("ParamType::{}", p))
                    .collect();

                let s = format!(
                    "Enum(EnumVariants::new(vec![{}]).unwrap())",
                    inner_strings.join(",")
                );
                write!(f, "{}", s)
            }
            ParamType::Tuple(inner) => {
                let inner_strings: Vec<String> =
                    inner.iter().map(|p| format!("ParamType::{}", p)).collect();

                let s = format!("Tuple(vec![{}])", inner_strings.join(","));
                write!(f, "{}", s)
            }
            ParamType::Unit => write! {f, "Unit"},
            ParamType::Generic(name) => write! {f, "{}", name},
            _ => {
                write!(f, "{:?}", self)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnumVariants {
    variants: Vec<ParamType>,
}

impl EnumVariants {
    pub fn new(variants: Vec<ParamType>) -> Result<EnumVariants, NoVariants> {
        if !variants.is_empty() {
            Ok(EnumVariants { variants })
        } else {
            Err(NoVariants)
        }
    }

    pub fn param_types(&self) -> &Vec<ParamType> {
        &self.variants
    }

    pub fn only_units_inside(&self) -> bool {
        self.variants
            .iter()
            .all(|variant| *variant == ParamType::Unit)
    }

    /// Calculates how many WORDs are needed to encode an enum.
    pub fn compute_encoding_width_of_enum(&self) -> usize {
        if self.only_units_inside() {
            return ENUM_DISCRIMINANT_WORD_WIDTH;
        }
        self.param_types()
            .iter()
            .map(|p| p.compute_encoding_width())
            .max()
            .map(|width| width + ENUM_DISCRIMINANT_WORD_WIDTH)
            .expect(
                "Will never panic because EnumVariants must have at least one variant inside it!",
            )
    }
    /// Determines the padding needed for the provided enum variant (based on the width of the
    /// biggest variant) and returns it.
    pub fn compute_padding_amount(&self, variant_param_type: &ParamType) -> usize {
        let biggest_variant_width =
            self.compute_encoding_width_of_enum() - ENUM_DISCRIMINANT_WORD_WIDTH;
        let variant_width = variant_param_type.compute_encoding_width();
        (biggest_variant_width - variant_width) * WORD_SIZE
    }
}

#[derive(ThisError, Debug)]
pub struct NoVariants;

impl fmt::Display for NoVariants {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "An Enum must have variants!")
    }
}

impl From<NoVariants> for Error {
    fn from(err: NoVariants) -> Self {
        Error::InvalidType(format!("{}", err))
    }
}

#[cfg(test)]
mod tests {
    const WIDTH_OF_B256: usize = 4;
    const WIDTH_OF_U32: usize = 1;
    const WIDTH_OF_BOOL: usize = 1;
    use super::*;

    #[test]
    fn array_size_dependent_on_num_of_elements() {
        const NUM_ELEMENTS: usize = 11;
        let param = ParamType::Array(Box::new(ParamType::B256), NUM_ELEMENTS);

        let width = param.compute_encoding_width();

        let expected = NUM_ELEMENTS * WIDTH_OF_B256;
        assert_eq!(expected, width);
    }

    #[test]
    fn string_size_dependent_on_num_of_elements() {
        const NUM_ASCII_CHARS: usize = 9;
        let param = ParamType::String(NUM_ASCII_CHARS);

        let width = param.compute_encoding_width();

        // 2 WORDS or 16 B are enough to fit 9 ascii chars
        assert_eq!(2, width);
    }

    #[test]
    fn structs_are_just_all_elements_combined() {
        let inner_struct = ParamType::Struct(vec![ParamType::U32, ParamType::U32]);

        let a_struct = ParamType::Struct(vec![ParamType::B256, ParamType::Bool, inner_struct]);

        let width = a_struct.compute_encoding_width();

        const INNER_STRUCT_WIDTH: usize = WIDTH_OF_U32 * 2;
        const EXPECTED_WIDTH: usize = WIDTH_OF_B256 + WIDTH_OF_BOOL + INNER_STRUCT_WIDTH;
        assert_eq!(EXPECTED_WIDTH, width);
    }

    #[test]
    fn enums_are_as_big_as_their_biggest_variant_plus_a_word() -> Result<(), Error> {
        let inner_struct = ParamType::Struct(vec![ParamType::B256]);
        let param = ParamType::Enum(EnumVariants::new(vec![ParamType::U32, inner_struct])?);

        let width = param.compute_encoding_width();

        const INNER_STRUCT_SIZE: usize = WIDTH_OF_B256;
        const EXPECTED_WIDTH: usize = INNER_STRUCT_SIZE + 1;
        assert_eq!(EXPECTED_WIDTH, width);
        Ok(())
    }

    #[test]
    fn tuples_are_just_all_elements_combined() {
        let inner_tuple = ParamType::Tuple(vec![ParamType::B256]);
        let param = ParamType::Tuple(vec![ParamType::U32, inner_tuple]);

        let width = param.compute_encoding_width();

        const INNER_TUPLE_WIDTH: usize = WIDTH_OF_B256;
        const EXPECTED_WIDTH: usize = WIDTH_OF_U32 + INNER_TUPLE_WIDTH;
        assert_eq!(EXPECTED_WIDTH, width);
    }
}
