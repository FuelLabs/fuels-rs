pub fn encode_fn_selector(name: &str) -> Vec<u8> {
    let bytes = name.as_bytes().to_vec();
    let len = bytes.len() as u64;

    [len.to_be_bytes().to_vec(), bytes].concat()
}

/// This uses the default `EncoderConfig` configuration.
#[macro_export]
macro_rules! calldata {
    ( $($arg: expr),* ) => {
        ::fuels::core::codec::ABIEncoder::default().encode(&[$(::fuels::core::traits::Tokenizable::into_token($arg)),*])
    }
}

pub use calldata;
