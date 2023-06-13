use std::{
    fmt::{Debug, Display, Formatter},
    io,
};

pub struct Error(pub String);

impl Error {
    pub fn combine<T: Into<Self>>(self, err: T) -> Self {
        error!("{} {}", self.0, err.into().0)
    }
}

#[macro_export]
macro_rules! error {
   ($fmt_str: literal $(,$arg: expr)*) => {$crate::error::Error(format!($fmt_str,$($arg),*))}
}

pub use error;

pub type Result<T> = std::result::Result<T, Error>;

impl Debug for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for Error {}

macro_rules! impl_from {
    ($($err_type:ty),*) => {
        $(
            impl From<$err_type> for self::Error {
                fn from(err: $err_type) -> Self {
                    Self(err.to_string())
                }
            }
        )*
    }
}

impl_from!(
    serde_json::Error,
    io::Error,
    proc_macro2::LexError,
    fuel_abi_types::error::Error
);
