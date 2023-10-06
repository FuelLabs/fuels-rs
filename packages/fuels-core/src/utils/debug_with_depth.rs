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
            ParamType::Struct { fields, generics } => f
                .debug_struct("Struct")
                .field(
                    "fields",
                    &fields
                        .iter()
                        .map(|field| self.descend(field))
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
            ParamType::Enum { variants, generics } => f
                .debug_struct("Enum")
                .field(
                    "variants",
                    &variants
                        .param_types()
                        .iter()
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
