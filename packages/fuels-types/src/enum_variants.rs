use crate::constants::{ENUM_DISCRIMINANT_WORD_WIDTH, WORD_SIZE};
use crate::param_types::ParamType;
use std::fmt;
use thiserror::Error as ThisError;

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
