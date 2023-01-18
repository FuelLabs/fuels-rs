use std::{
    fmt::{Debug, Display, Formatter},
    io,
};

pub(crate) struct Error(pub(crate) String);
pub(crate) type Result<T> = std::result::Result<T, Error>;

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
    };
}

impl_from!(serde_json::Error, io::Error, proc_macro2::LexError);

// impl From<io::Error> for Error {
//     fn from(err: io::Error) -> Self {
//         Error(err.to_string())
//     }
// }
//
// impl From<LexError> for Error {
//     fn from(err: LexError) -> Self {
//         Error(err.to_string())
//     }
// }

// impl From<serde_json::Error> for Error {
//     fn from(err: serde_json::Error) -> Self {
//         Error(err.to_string())
//     }
// }
