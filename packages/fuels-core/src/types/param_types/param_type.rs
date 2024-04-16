use crate::types::errors::{error, Result};

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
}
