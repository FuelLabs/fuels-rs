use crate::{
    constants::ENUM_DISCRIMINANT_BYTE_WIDTH,
    types::{
        errors::{error, Result},
        param_types::ParamType,
    },
    utils::checked_round_up_to_word_alignment,
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

    pub fn param_type_of_variant(&self, discriminant: u64) -> Result<&ParamType> {
        self.param_types.get(discriminant as usize).ok_or_else(|| {
            error!(
                InvalidData,
                "Discriminant '{discriminant}' doesn't point to any variant: {:?}",
                self.param_types()
            )
        })
    }

    pub fn heap_type_variant(&self) -> Option<(u64, &ParamType)> {
        self.param_types()
            .iter()
            .enumerate()
            .find_map(|(d, p)| p.is_extra_receipt_needed(false).then_some((d as u64, p)))
    }

    pub fn only_units_inside(&self) -> bool {
        self.param_types
            .iter()
            .all(|param_type| *param_type == ParamType::Unit)
    }

    /// Calculates how many bytes are needed to encode an enum.
    pub fn compute_enum_width_in_bytes(&self) -> Result<usize> {
        if self.only_units_inside() {
            return Ok(ENUM_DISCRIMINANT_BYTE_WIDTH);
        }

        let width = self.param_types().iter().try_fold(0, |a, p| -> Result<_> {
            let size = p.compute_encoding_in_bytes()?;
            Ok(a.max(size))
        })?;

        checked_round_up_to_word_alignment(width)?
            .checked_add(ENUM_DISCRIMINANT_BYTE_WIDTH)
            .ok_or_else(|| error!(InvalidType, "Enum variants are too wide"))
    }

    /// Determines the padding needed for the provided enum variant (based on the width of the
    /// biggest variant) and returns it.
    pub fn compute_padding_amount_in_bytes(&self, variant_param_type: &ParamType) -> Result<usize> {
        let enum_width = self.compute_enum_width_in_bytes()?;
        let biggest_variant_width = enum_width - ENUM_DISCRIMINANT_BYTE_WIDTH;
        let variant_width = variant_param_type.compute_encoding_in_bytes()?;
        Ok(biggest_variant_width - variant_width)
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
