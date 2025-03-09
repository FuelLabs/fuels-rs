mod signature_utils;

#[cfg(any(feature = "aws-kms-signer", feature = "google-kms-signer"))]
mod kms_wallet;

#[cfg(feature = "aws-kms-signer")]
mod aws_signer;

#[cfg(feature = "google-kms-signer")]
mod google_signer;

#[cfg(any(feature = "aws-kms-signer", feature = "google-kms-signer"))]
pub use kms_wallet::*;

#[cfg(feature = "aws-kms-signer")]
pub use {aws_config, aws_sdk_kms, aws_signer::*};

#[cfg(feature = "google-kms-signer")]
pub use {google_cloud_kms, google_signer::*};
