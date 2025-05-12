# Using KMS Wallets

Key Management Service (KMS) is a robust and secure solution for managing cryptographic keys for your Fuel wallets. Instead of keeping private keys on your local system, KMS Wallets leverage secure infrastructure to handle both key storage and signing operations.

The SDK provides signers for AWS and Google KMS.

Below is an example of how to initialize a wallet with a AWS KMS signer:

```rust,ignore
{{#include ../../../e2e/tests/aws.rs:use_kms_wallet}}
```
