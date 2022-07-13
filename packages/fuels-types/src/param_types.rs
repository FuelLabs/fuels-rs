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
        match &*self {
            Self::Unit | Self::U8 | Self::U16 | Self::U32 | Self::U64 | Self::Bool => {
                ReturnLocation::Return
            }

            _ => ReturnLocation::ReturnData,
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
}
