use fuels_core::types::errors::{error, Error};

pub fn prepend_msg<'a>(msg: impl AsRef<str> + 'a) -> impl Fn(Error) -> Error + 'a {
    move |err| match err {
        Error::IO(orig_msg) => {
            error!(IO, "{}: {}", msg.as_ref(), orig_msg)
        }
        Error::Codec(orig_msg) => {
            error!(Codec, "{}: {}", msg.as_ref(), orig_msg)
        }
        Error::Transaction(reason) => Error::Transaction(reason),
        Error::Provider(orig_msg) => {
            error!(Provider, "{}: {}", msg.as_ref(), orig_msg)
        }
        Error::Other(orig_msg) => {
            error!(Other, "{}: {}", msg.as_ref(), orig_msg)
        }
    }
}
