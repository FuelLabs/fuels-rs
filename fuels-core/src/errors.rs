use core::fmt;
use core::str::Utf8Error;

#[derive(Debug)]
pub enum CodecError {
    InvalidData,
    Utf8Error(Utf8Error),
}

impl fmt::Display for CodecError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl From<Utf8Error> for CodecError {
    fn from(e: Utf8Error) -> CodecError {
        CodecError::Utf8Error(e)
    }
}
