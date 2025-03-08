mod signature_utils;
pub use signature_utils::*;

#[cfg(feature = "accounts-aws-kms-signer")]
pub mod aws;
#[cfg(feature = "accounts-google-kms-signer")]
pub mod google;

#[cfg(any(
    feature = "accounts-aws-kms-signer",
    feature = "accounts-google-kms-signer"
))]
mod kms_wallets;
#[cfg(any(
    feature = "accounts-aws-kms-signer",
    feature = "accounts-google-kms-signer"
))]
pub use kms_wallets::*;

#[cfg(feature = "accounts-aws-kms-signer")]
pub use aws::*;
#[cfg(feature = "accounts-aws-kms-signer")]
pub use aws_config;
#[cfg(feature = "accounts-aws-kms-signer")]
pub use aws_sdk_kms;

#[cfg(feature = "accounts-google-kms-signer")]
pub use google::*;
#[cfg(feature = "accounts-google-kms-signer")]
pub use google_cloud_kms;
