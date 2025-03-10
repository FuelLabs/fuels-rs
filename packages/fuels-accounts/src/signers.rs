use fuel_crypto::coins_bip32::path::DerivationPath;

pub const DEFAULT_DERIVATION_PATH: &str = "m/44'/1179993420'/0'/0/0";
#[cfg(feature = "signer-aws-kms")]
pub mod aws_kms;
pub mod fake;
pub mod private_key;
