use crate::{
    constants::{ENUM_DISCRIMINANT_WORD_WIDTH, WORD_SIZE},
    types::{
        errors::{error, Error, Result},
        param_types::ParamType,
    },
};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct EnumVariants {
    param_types: Vec<ParamType>,
}

impl EnumVariants {
    pub fn new(param_types: Vec<ParamType>) -> Result<EnumVariants> {
        if param_types.is_empty() {
            return Err(error!(InvalidData, "Enum variants can not be empty!"));
        }
        Ok(EnumVariants { param_types })
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

    pub fn heap_type_variant(&self) -> Option<(u8, &ParamType)> {
        self.param_types()
            .iter()
            .enumerate()
            .find_map(|(d, p)| p.is_extra_receipt_needed(false).then_some((d as u8, p)))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_heap_type_variant_discriminant() -> Result<()> {
        let param_types = vec![
            ParamType::U64,
            ParamType::Bool,
            ParamType::Vector(Box::from(ParamType::U64)),
        ];
        let variants = EnumVariants::new(param_types)?;
        assert_eq!(variants.heap_type_variant().unwrap().0, 2);

        let param_types = vec![
            ParamType::Vector(Box::from(ParamType::U64)),
            ParamType::U64,
            ParamType::Bool,
        ];
        let variants = EnumVariants::new(param_types)?;
        assert_eq!(variants.heap_type_variant().unwrap().0, 0);

        let param_types = vec![ParamType::U64, ParamType::Bool];
        let variants = EnumVariants::new(param_types)?;
        assert!(variants.heap_type_variant().is_none());
        Ok(())
    }
}
