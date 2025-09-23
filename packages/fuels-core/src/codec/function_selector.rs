pub use fuels_code_gen::utils::encode_fn_selector;

/// This uses the default `EncoderConfig` configuration.
#[macro_export]
macro_rules! calldata {
    ( $($arg: expr),* ) => {
        ::fuels::core::codec::ABIEncoder::default().encode(&[$(::fuels::core::traits::Tokenizable::into_token($arg)),*])
    }
}

pub use calldata;
