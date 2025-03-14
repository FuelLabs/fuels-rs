pub mod derivation {
    pub const BIP44_PURPOSE: &str = "44'";
    pub const COIN_TYPE: &str = "1179993420'";
    pub const DEFAULT_DERIVATION_PATH: &str = "m/44'/1179993420'/0'/0/0";
}

#[cfg(any(feature = "signer-aws-kms", feature = "signer-google-kms"))]
pub mod kms;

pub mod fake;
pub mod private_key;
