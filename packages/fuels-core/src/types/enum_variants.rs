use crate::{
    constants::{ENUM_DISCRIMINANT_WORD_WIDTH, WORD_SIZE},
    types::{
        errors::{error, Error, Result},
        param_types::ParamType,
    },
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnumVariants {
    param_types: Vec<ParamType>,
}

impl EnumVariants {
    pub fn new(param_types: Vec<ParamType>) -> Result<EnumVariants> {
        if !param_types.is_empty() {
            Ok(EnumVariants { param_types })
        } else {
            Err(error!(InvalidData, "Enum variants can not be empty!"))
        }
    }

    pub fn param_types(&self) -> &[ParamType] {
        &self.param_types
    }

    pub fn param_type_of_variant(&self, discriminant: u8) -> Result<&ParamType> {
        self.param_types.get(discriminant as usize).ok_or_else(|| {
            error!(
                InvalidData,
                "Discriminant '{discriminant}' doesn't point to any variant: {:?}",
                self.param_types()
            )
        })
    }

    pub fn only_units_inside(&self) -> bool {
        self.param_types
            .iter()
            .all(|param_type| *param_type == ParamType::Unit)
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
