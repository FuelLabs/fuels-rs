mod signature_utils;

#[cfg(feature = "accounts-aws-kms-signer")]
mod aws;
#[cfg(feature = "accounts-google-kms-signer")]
mod google;

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
pub use {aws::*, aws_config, aws_sdk_kms};

#[cfg(feature = "accounts-google-kms-signer")]
pub use {google::*, google_cloud_kms};
