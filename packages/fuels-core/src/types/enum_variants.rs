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
        if param_types.iter().filter(|p| p.is_vm_heap_type()).count() > 1 {
            Err(error!(
                InvalidData,
                "Enum variants can only contain one heap type"
            ))
        } else {
            Ok(EnumVariants { param_types })
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

    pub fn get_heap_type_variant_discriminant(&self) -> Result<u8> {
        for (discriminant, param) in self.param_types.iter().enumerate() {
            if param.is_vm_heap_type() {
                return Ok(discriminant as u8);
            }
        }
        Err(error!(
            InvalidData,
            "There are no heap types inside {:?}", self
        ))
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
    fn test_enum_variants_can_have_only_one_heap_type() -> Result<()> {
        let mut param_types = vec![
            ParamType::U64,
            ParamType::Bool,
            ParamType::Vector(Box::from(ParamType::U64)),
        ];
        // it works if there is only one heap type
        let _variants = EnumVariants::new(param_types.clone())?;
        param_types.append(&mut vec![ParamType::Bytes]);

        let error = EnumVariants::new(param_types).expect_err("Should have failed");
        let expected_error = format!("Invalid data: Enum variants can only contain one heap type");
        assert_eq!(error.to_string(), expected_error);

        Ok(())
    }
}
