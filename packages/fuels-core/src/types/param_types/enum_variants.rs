use crate::{
    constants::ENUM_DISCRIMINANT_BYTE_WIDTH,
    types::{
        errors::{error, Result},
        param_types::{NamedParamType, ParamType},
    },
    utils::checked_round_up_to_word_alignment,
};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct EnumVariants {
    variants: Vec<NamedParamType>,
}

impl EnumVariants {
    pub fn new(variants: Vec<NamedParamType>) -> Result<EnumVariants> {
        if variants.is_empty() {
            return Err(error!(Other, "enum variants cannot be empty!"));
        }

        Ok(EnumVariants { variants })
    }

    pub fn variants(&self) -> &Vec<NamedParamType> {
        &self.variants
    }

    pub fn param_types(&self) -> impl Iterator<Item = &ParamType> {
        self.variants.iter().map(|(_, param_type)| param_type)
    }

    pub fn select_variant(&self, discriminant: u64) -> Result<&NamedParamType> {
        self.variants.get(discriminant as usize).ok_or_else(|| {
            error!(
                Other,
                "discriminant `{discriminant}` doesn't point to any variant: {:?}",
                self.variants()
            )
        })
    }

    pub fn heap_type_variant(&self) -> Option<(u64, &ParamType)> {
        self.param_types()
            .enumerate()
            .find_map(|(d, p)| p.is_extra_receipt_needed(false).then_some((d as u64, p)))
    }

    pub fn only_units_inside(&self) -> bool {
        self.variants
            .iter()
            .all(|(_, param_type)| *param_type == ParamType::Unit)
    }

    /// Calculates how many bytes are needed to encode an enum.
    pub fn compute_enum_width_in_bytes(&self) -> Result<usize> {
        if self.only_units_inside() {
            return Ok(ENUM_DISCRIMINANT_BYTE_WIDTH);
        }

        let width = self.param_types().try_fold(0, |a, p| -> Result<_> {
            let size = p.compute_encoding_in_bytes()?;
            Ok(a.max(size))
        })?;

        checked_round_up_to_word_alignment(width)?
            .checked_add(ENUM_DISCRIMINANT_BYTE_WIDTH)
            .ok_or_else(|| error!(Other, "enum variants are too wide"))
    }

    /// Determines the padding needed for the provided enum variant (based on the width of the
    /// biggest variant) and returns it.
    pub fn compute_padding_amount_in_bytes(&self, variant_param_type: &ParamType) -> Result<usize> {
        let enum_width = self.compute_enum_width_in_bytes()?;
        // No need to use checked arithmetics since we called `compute_enum_width_in_bytes`
        let biggest_variant_width = enum_width - ENUM_DISCRIMINANT_BYTE_WIDTH;
        let variant_width = variant_param_type.compute_encoding_in_bytes()?;
        Ok(biggest_variant_width - variant_width)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::to_named;

    #[test]
    fn test_get_heap_type_variant_discriminant() -> Result<()> {
        {
            let variants = to_named(&[
                ParamType::U64,
                ParamType::Bool,
                ParamType::Vector(Box::from(ParamType::U64)),
            ]);
            let enum_variants = EnumVariants::new(variants)?;

            assert_eq!(enum_variants.heap_type_variant().unwrap().0, 2);
        }
        {
            let variants = to_named(&[
                ParamType::Vector(Box::from(ParamType::U64)),
                ParamType::U64,
                ParamType::Bool,
            ]);
            let enum_variants = EnumVariants::new(variants)?;

            assert_eq!(enum_variants.heap_type_variant().unwrap().0, 0);
        }
        {
            let variants = to_named(&[ParamType::U64, ParamType::Bool]);
            let enum_variants = EnumVariants::new(variants)?;

            assert!(enum_variants.heap_type_variant().is_none());
        }

        Ok(())
    }
}
