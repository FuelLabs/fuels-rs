use std::fmt;

use crate::types::param_types::ParamType;

/// Allows `Debug` formatting of arbitrary-depth nested `ParamTypes` by
/// omitting the details of inner types if max depth is exceeded.
pub(crate) struct DebugWithDepth<'a> {
    param_type: &'a ParamType,
    depth_left: usize,
}

impl<'a> DebugWithDepth<'a> {
    pub(crate) fn new(param_type: &'a ParamType, depth_left: usize) -> Self {
        Self {
            param_type,
            depth_left,
        }
    }

    fn descend(&'a self, param_type: &'a ParamType) -> Self {
        Self {
            param_type,
            depth_left: self.depth_left - 1,
        }
    }
}

impl<'a> fmt::Debug for DebugWithDepth<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.depth_left == 0 {
            return write!(f, "...");
        }

        match &self.param_type {
            ParamType::Array(inner, size) => f
                .debug_tuple("Array")
                .field(&self.descend(inner))
                .field(&size)
                .finish(),
            ParamType::Struct {
                fields,
                generics,
                name,
            } => f
                .debug_struct(name)
                .field(
                    "fields",
                    &fields
                        .iter()
                        .map(|(_, field)| self.descend(field))
                        .collect::<Vec<_>>(),
                )
                .field(
                    "generics",
                    &generics
                        .iter()
                        .map(|generic| self.descend(generic))
                        .collect::<Vec<_>>(),
                )
                .finish(),
            ParamType::Enum {
                enum_variants,
                generics,
                name,
            } => f
                .debug_struct(name)
                .field(
                    "variants",
                    &enum_variants
                        .param_types()
                        .map(|variant| self.descend(variant))
                        .collect::<Vec<_>>(),
                )
                .field(
                    "generics",
                    &generics
                        .iter()
                        .map(|generic| self.descend(generic))
                        .collect::<Vec<_>>(),
                )
                .finish(),
            ParamType::Tuple(inner) => f
                .debug_tuple("Tuple")
                .field(
                    &inner
                        .iter()
                        .map(|param_type| self.descend(param_type))
                        .collect::<Vec<_>>(),
                )
                .finish(),
            ParamType::Vector(inner) => {
                f.debug_tuple("Vector").field(&self.descend(inner)).finish()
            }
            _ => write!(f, "{:?}", self.param_type),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{codec::DecoderConfig, to_named, types::errors::Result};

    #[test]
    fn validate_is_decodable_complex_types_containing_bytes() -> Result<()> {
        let param_types_containing_bytes = vec![ParamType::Bytes, ParamType::U64, ParamType::Bool];
        let param_types_no_bytes = vec![ParamType::U64, ParamType::U32];
        let max_depth = DecoderConfig::default().max_depth;
        let nested_heap_type_error_message = |p: ParamType| {
            format!(
                "codec: type `{:?}` is not decodable: nested heap types are currently not \
        supported except in enums",
                DebugWithDepth::new(&p, max_depth)
            )
        };
        let cannot_be_decoded = |p: ParamType| {
            assert_eq!(
                p.validate_is_decodable(max_depth)
                    .expect_err(&format!("should not be decodable: {:?}", p))
                    .to_string(),
                nested_heap_type_error_message(p)
            )
        };
        let can_be_decoded = |p: ParamType| p.validate_is_decodable(max_depth).is_ok();

        can_be_decoded(ParamType::Array(Box::new(ParamType::U64), 10usize));
        cannot_be_decoded(ParamType::Array(Box::new(ParamType::Bytes), 10usize));

        can_be_decoded(ParamType::Vector(Box::new(ParamType::U64)));
        cannot_be_decoded(ParamType::Vector(Box::new(ParamType::Bytes)));

        can_be_decoded(ParamType::Struct {
            name: "".to_string(),
            fields: to_named(&param_types_no_bytes),
            generics: param_types_no_bytes.clone(),
        });
        cannot_be_decoded(ParamType::Struct {
            name: "".to_string(),
            fields: to_named(&param_types_containing_bytes),
            generics: param_types_no_bytes.clone(),
        });

        can_be_decoded(ParamType::Tuple(param_types_no_bytes.clone()));
        cannot_be_decoded(ParamType::Tuple(param_types_containing_bytes.clone()));

        Ok(())
    }

    #[test]
    fn validate_is_decodable_complex_types_containing_string() -> Result<()> {
        let max_depth = DecoderConfig::default().max_depth;
        let base_string = ParamType::String;
        let param_types_no_nested_string = vec![ParamType::U64, ParamType::U32];
        let param_types_nested_string = vec![ParamType::Unit, ParamType::Bool, base_string.clone()];
        let nested_heap_type_error_message = |p: ParamType| {
            format!(
                "codec: type `{:?}` is not decodable: nested heap types \
        are currently not supported except in enums",
                DebugWithDepth::new(&p, max_depth)
            )
        };
        let cannot_be_decoded = |p: ParamType| {
            assert_eq!(
                p.validate_is_decodable(max_depth)
                    .expect_err(&format!("should not be decodable: {:?}", p))
                    .to_string(),
                nested_heap_type_error_message(p)
            )
        };
        let can_be_decoded = |p: ParamType| p.validate_is_decodable(max_depth).is_ok();

        can_be_decoded(base_string.clone());
        cannot_be_decoded(ParamType::Vector(Box::from(base_string.clone())));

        can_be_decoded(ParamType::Array(Box::from(ParamType::U8), 10));
        cannot_be_decoded(ParamType::Array(Box::from(base_string), 10));

        can_be_decoded(ParamType::Tuple(param_types_no_nested_string.clone()));
        cannot_be_decoded(ParamType::Tuple(param_types_nested_string.clone()));

        can_be_decoded(ParamType::Struct {
            name: "".to_string(),
            fields: to_named(&param_types_no_nested_string),
            generics: param_types_no_nested_string.clone(),
        });

        can_be_decoded(ParamType::Struct {
            name: "".to_string(),
            fields: to_named(&param_types_no_nested_string),
            generics: param_types_nested_string.clone(),
        });

        cannot_be_decoded(ParamType::Struct {
            name: "".to_string(),
            fields: to_named(&param_types_nested_string),
            generics: param_types_no_nested_string.clone(),
        });

        Ok(())
    }

    #[test]
    fn validate_is_decodable_complex_types_containing_vector() -> Result<()> {
        let max_depth = DecoderConfig::default().max_depth;
        let param_types_containing_vector = vec![
            ParamType::Vector(Box::new(ParamType::U32)),
            ParamType::U64,
            ParamType::Bool,
        ];
        let param_types_no_vector = vec![ParamType::U64, ParamType::U32];
        let nested_heap_type_error_message = |p: ParamType| {
            format!(
                "codec: type `{:?}` is not decodable: nested heap types \
        are currently not supported except in enums",
                DebugWithDepth::new(&p, max_depth)
            )
        };

        let cannot_be_decoded = |p: ParamType| {
            assert_eq!(
                p.validate_is_decodable(max_depth)
                    .expect_err(&format!("should not be decodable: {:?}", p))
                    .to_string(),
                nested_heap_type_error_message(p)
            )
        };
        let can_be_decoded = |p: ParamType| p.validate_is_decodable(max_depth).is_ok();

        can_be_decoded(ParamType::Array(Box::new(ParamType::U64), 10usize));
        cannot_be_decoded(ParamType::Array(
            Box::new(ParamType::Vector(Box::new(ParamType::U8))),
            10usize,
        ));

        can_be_decoded(ParamType::Vector(Box::new(ParamType::U64)));
        cannot_be_decoded(ParamType::Vector(Box::new(ParamType::Vector(Box::new(
            ParamType::U8,
        )))));

        can_be_decoded(ParamType::Struct {
            name: "".to_string(),
            fields: to_named(&param_types_no_vector),
            generics: param_types_no_vector.clone(),
        });
        cannot_be_decoded(ParamType::Struct {
            name: "".to_string(),
            generics: param_types_no_vector.clone(),
            fields: to_named(&param_types_containing_vector),
        });

        can_be_decoded(ParamType::Tuple(param_types_no_vector.clone()));
        cannot_be_decoded(ParamType::Tuple(param_types_containing_vector.clone()));

        Ok(())
    }
}
